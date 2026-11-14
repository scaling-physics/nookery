//! OCR on macOS with Apple Vision (`VNRecognizeTextRequest`). Photos and
//! scanned PDF pages are decoded/rendered with ImageIO / CoreGraphics.

use crate::{rows, ExtractError, Word};
use objc2::rc::{autoreleasepool, Retained};
use objc2::AnyThread;
use objc2_core_foundation::{CFData, CFNumber, CFRetained, CGFloat, CGPoint, CGRect, CGSize};
use objc2_core_graphics::{
    CGBitmapInfo, CGColorSpace, CGContext, CGDataProvider, CGImage, CGImageAlphaInfo, CGPDFBox,
    CGPDFDocument,
};
use objc2_foundation::{NSArray, NSDictionary, NSString};
use objc2_image_io::{kCGImagePropertyOrientation, CGImagePropertyOrientation, CGImageSource};
use objc2_vision::{
    VNImageOption, VNImageRequestHandler, VNRecognizeTextRequest, VNRequest,
    VNRequestTextRecognitionLevel,
};
use std::ffi::c_void;
use std::ptr::NonNull;

/// ~300 dpi for a scanned PDF page (PDF user space is 72 dpi).
const PDF_SCALE: f64 = 300.0 / 72.0;
/// Longest side of a rendered PDF page, in pixels.
const MAX_PIXELS: f64 = 3000.0;

extern "C-unwind" {
    fn CGBitmapContextCreate(
        data: *mut c_void,
        width: usize,
        height: usize,
        bits_per_component: usize,
        bytes_per_row: usize,
        space: Option<&CGColorSpace>,
        bitmap_info: u32,
    ) -> Option<NonNull<CGContext>>;
}

fn orientation_of(src: &CGImageSource) -> CGImagePropertyOrientation {
    use CGImagePropertyOrientation as O;
    let Some(props) = (unsafe { src.properties_at_index(0, None) }) else {
        return O::Up;
    };
    let key: &objc2_core_foundation::CFString = unsafe { kCGImagePropertyOrientation };
    let raw = unsafe { props.value(key as *const _ as *const c_void) };
    if raw.is_null() {
        return O::Up;
    }
    let num = unsafe { &*(raw as *const CFNumber) };
    match num.as_i32().unwrap_or(1) {
        2 => O::UpMirrored,
        3 => O::Down,
        4 => O::DownMirrored,
        5 => O::LeftMirrored,
        6 => O::Right,
        7 => O::RightMirrored,
        8 => O::Left,
        _ => O::Up,
    }
}

pub fn ocr_image(bytes: &[u8]) -> Result<String, ExtractError> {
    autoreleasepool(|_| {
        let data = CFData::from_bytes(bytes);
        let src = unsafe { CGImageSource::with_data(&data, None) }
            .ok_or_else(|| ExtractError::Image("not a readable image".into()))?;
        let image = unsafe { src.image_at_index(0, None) }
            .ok_or_else(|| ExtractError::Image("not a readable image".into()))?;
        let orientation = orientation_of(&src);
        recognise(&image, orientation)
    })
}

pub fn ocr_pdf(bytes: &[u8]) -> Result<(String, u32), ExtractError> {
    autoreleasepool(|_| {
        let data = CFData::from_bytes(bytes);
        let provider = CGDataProvider::with_cf_data(Some(&data))
            .ok_or_else(|| ExtractError::Pdf("unreadable".into()))?;
        let doc = CGPDFDocument::with_provider(Some(&provider))
            .ok_or_else(|| ExtractError::Pdf("unreadable".into()))?;
        if CGPDFDocument::is_encrypted(Some(&doc)) && !CGPDFDocument::is_unlocked(Some(&doc)) {
            return Err(ExtractError::Pdf("password protected".into()));
        }
        let n = CGPDFDocument::number_of_pages(Some(&doc));
        let mut out = Vec::new();
        for i in 1..=n {
            let image = autoreleasepool(|_| render_page(&doc, i))?;
            let text = autoreleasepool(|_| recognise(&image, CGImagePropertyOrientation::Up))?;
            if !text.trim().is_empty() {
                out.push(text);
            }
        }
        Ok((out.join("\n\n"), n as u32))
    })
}

fn render_page(doc: &CGPDFDocument, number: usize) -> Result<CFRetained<CGImage>, ExtractError> {
    let page = CGPDFDocument::page(Some(doc), number)
        .ok_or_else(|| ExtractError::Pdf(format!("page {number} missing")))?;
    let media = objc2_core_graphics::CGPDFPage::box_rect(Some(&page), CGPDFBox::CropBox);
    let (pw, ph) = (media.size.width.max(1.0), media.size.height.max(1.0));
    let rotated = matches!(
        objc2_core_graphics::CGPDFPage::rotation_angle(Some(&page)).rem_euclid(360),
        90 | 270
    );
    let (vw, vh) = if rotated { (ph, pw) } else { (pw, ph) };
    let scale = PDF_SCALE.min(MAX_PIXELS / vw.max(vh));
    let (w, h) = ((vw * scale).ceil() as usize, (vh * scale).ceil() as usize);
    let space = CGColorSpace::new_device_rgb()
        .ok_or_else(|| ExtractError::Pdf("no colour space".into()))?;
    let info = CGBitmapInfo(CGImageAlphaInfo::NoneSkipLast.0);
    let ctx =
        unsafe { CGBitmapContextCreate(std::ptr::null_mut(), w, h, 8, 0, Some(&space), info.0) }
            .map(|p| unsafe { CFRetained::from_raw(p) })
            .ok_or_else(|| ExtractError::Pdf("can't allocate the page bitmap".into()))?;
    let full = CGRect {
        origin: CGPoint { x: 0.0, y: 0.0 },
        size: CGSize {
            width: w as CGFloat,
            height: h as CGFloat,
        },
    };
    CGContext::set_rgb_fill_color(Some(&ctx), 1.0, 1.0, 1.0, 1.0);
    CGContext::fill_rect(Some(&ctx), full);
    let t = objc2_core_graphics::CGPDFPage::drawing_transform(
        Some(&page),
        CGPDFBox::CropBox,
        full,
        0,
        true,
    );
    CGContext::concat_ctm(Some(&ctx), t);
    CGContext::draw_pdf_page(Some(&ctx), Some(&page));
    objc2_core_graphics::CGBitmapContextCreateImage(Some(&ctx))
        .ok_or_else(|| ExtractError::Pdf("can't render the page".into()))
}

fn recognise(
    image: &CGImage,
    orientation: CGImagePropertyOrientation,
) -> Result<String, ExtractError> {
    let request = VNRecognizeTextRequest::new();
    request.setRecognitionLevel(VNRequestTextRecognitionLevel::Accurate);
    let langs =
        NSArray::from_retained_slice(&[NSString::from_str("de-DE"), NSString::from_str("en-US")]);
    request.setRecognitionLanguages(&langs);
    request.setUsesLanguageCorrection(false);

    let options: Retained<NSDictionary<VNImageOption, objc2::runtime::AnyObject>> =
        NSDictionary::new();
    let handler = unsafe {
        VNImageRequestHandler::initWithCGImage_orientation_options(
            VNImageRequestHandler::alloc(),
            image,
            orientation,
            &options,
        )
    };
    let base: Retained<VNRequest> = Retained::into_super(Retained::into_super(request.clone()));
    let reqs = NSArray::from_retained_slice(&[base]);
    handler
        .performRequests_error(&reqs)
        .map_err(|e| ExtractError::Image(e.localizedDescription().to_string()))?;

    let mut words = Vec::new();
    if let Some(results) = request.results() {
        for obs in results.iter() {
            let cands = obs.topCandidates(1);
            let Some(top) = cands.firstObject() else {
                continue;
            };
            let text = top.string().to_string();
            let b = unsafe { obs.boundingBox() };
            // Normalised, origin bottom-left -> top-left, y down.
            words.push(Word {
                text,
                x0: b.origin.x as f32,
                x1: (b.origin.x + b.size.width) as f32,
                y0: (1.0 - b.origin.y - b.size.height) as f32,
                y1: (1.0 - b.origin.y) as f32,
            });
        }
    }
    // The normalised box is in unit squares: scale to pixels so x and y
    // distances are comparable (the aspect ratio matters for skew).
    let (iw, ih) = (
        objc2_core_graphics::CGImage::width(Some(image)) as f32,
        objc2_core_graphics::CGImage::height(Some(image)) as f32,
    );
    let (iw, ih) = match orientation {
        CGImagePropertyOrientation::LeftMirrored
        | CGImagePropertyOrientation::Right
        | CGImagePropertyOrientation::RightMirrored
        | CGImagePropertyOrientation::Left => (ih, iw),
        _ => (iw, ih),
    };
    for w in &mut words {
        w.x0 *= iw;
        w.x1 *= iw;
        w.y0 *= ih;
        w.y1 *= ih;
    }
    Ok(rows(&words))
}
