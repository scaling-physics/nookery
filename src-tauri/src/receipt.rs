//! Receipt import: thin commands over `nookery-receipt` and the store.
//! Amounts are never taken from the UI: `import_receipt` re-parses the text.
//!
//! `import_receipt` reports a bad field as `BadInput("<field>")` for the
//! receipt as a whole (`total`, `store`, `text`) or `BadInput("<field>:<line>")`
//! for one line (`item:3`, `qty:3`, `location:3`, `bestBefore:3`, `line:3`),
//! where `<line>` is the line's `index` from `parse_receipt`.

use chrono::{DateTime, Local, NaiveDate, Utc};
use nookery_core::{parse_date, Alias, Location, Qty};
use nookery_extract::{ExtractError, Extracted, Method};
use nookery_receipt::{
    parse_text, propose, to_events, Chain, Confirmed, ImportError, LineKind, ParseError, Receipt,
};
use nookery_store::{ReceiptRecord, Store, StoreError};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;
use tauri::State;

use super::{bad, AppState, CommandError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptLineKind {
    Item,
    Deposit,
    DepositReturn,
    Discount,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProposalView {
    /// The learned product name, if any.
    pub item: Option<String>,
    pub qty: Option<String>,
    pub location: Option<String>,
    /// With discounts applied.
    pub price_cents: i32,
    /// True when the printed text matched a learned alias.
    pub known: bool,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptLineView {
    pub index: u32,
    pub kind: ReceiptLineKind,
    pub text: String,
    pub count: u32,
    pub weight: Option<String>,
    pub amount_cents: i32,
    pub proposal: Option<ProposalView>,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptView {
    pub id: String,
    /// Chain name ("REWE", "denn's"); None when it can't be told.
    pub store: Option<String>,
    pub date: Option<String>,
    pub total_cents: Option<i32>,
    pub lines_cents: i32,
    pub total_ok: bool,
    pub already_imported: bool,
    pub lines: Vec<ReceiptLineView>,
}

#[derive(Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptLineInput {
    pub line: u32,
    pub item: String,
    pub qty: String,
    pub location: String,
    pub best_before: Option<String>,
}

#[derive(Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptImportInput {
    pub text: String,
    pub store: Option<String>,
    /// Only the lines to track.
    pub lines: Vec<ReceiptLineInput>,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportResultView {
    pub added: u32,
    pub learned: u32,
}

/// A receipt file read into text (PDF text layer, OCR of a photo, or plain text).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptReadMethod {
    Text,
    PdfText,
    Ocr,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptFileView {
    /// Goes into the receipt text box, then through `parse_receipt`.
    pub text: String,
    pub method: ReceiptReadMethod,
    pub ms: u32,
    pub pages: u32,
    /// Which device did the reading ("this Mac").
    pub read_by: String,
}

fn unreadable(reason: &str) -> CommandError {
    CommandError::ReceiptUnreadable(reason.into())
}

/// `ExtractError` -> `ReceiptUnreadable(<reason code>)`.
pub fn extract_error(e: &ExtractError) -> CommandError {
    unreadable(match e {
        ExtractError::UnknownFormat => "unknown_format",
        ExtractError::TooLarge { .. } => "too_large",
        ExtractError::NoText => "no_text",
        ExtractError::OcrUnavailable => "ocr_unavailable",
        ExtractError::Pdf(_) => "pdf",
        ExtractError::Image(_) => "image",
    })
}

pub fn receipt_view(x: Extracted, read_by: &str) -> ReceiptFileView {
    ReceiptFileView {
        text: x.text,
        method: match x.method {
            Method::Text => ReceiptReadMethod::Text,
            Method::PdfText => ReceiptReadMethod::PdfText,
            Method::Ocr => ReceiptReadMethod::Ocr,
        },
        ms: x.ms.min(u32::MAX as u64) as u32,
        pages: x.pages,
        read_by: read_by.into(),
    }
}

/// Reads the file on this device (blocking; OCR is Apple Vision, Mac only).
#[cfg_attr(mobile, allow(dead_code))] // the phone goes through phone::read_receipt_via_mac
pub fn read_receipt_file_in(bytes: &[u8]) -> Result<ReceiptFileView, CommandError> {
    let x = nookery_extract::extract(bytes).map_err(|e| extract_error(&e))?;
    Ok(receipt_view(x, "this Mac"))
}

/// `file` = base64 of the file's bytes. No store lock: nothing is written.
#[tauri::command]
#[specta::specta]
pub async fn read_receipt_file(
    state: State<'_, AppState>,
    file: String,
) -> Result<ReceiptFileView, CommandError> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(file.trim())
        .map_err(|_| bad("file"))?;
    #[cfg(mobile)]
    {
        let x = crate::phone::read_receipt_via_mac(&state, &bytes).await?;
        Ok(receipt_view(x, "your Mac"))
    }
    #[cfg(not(mobile))]
    {
        let _ = &state;
        tauri::async_runtime::spawn_blocking(move || read_receipt_file_in(&bytes))
            .await
            .map_err(|_| CommandError::Internal)?
    }
}

/// Parses `text`; a chosen `store` overrides detection.
fn parse(text: &str, store: Option<&str>) -> Result<Receipt, CommandError> {
    let chain = match store.map(str::trim) {
        None | Some("") => None,
        Some(s) => Some(s.parse::<Chain>().map_err(|_| bad("store"))?),
    };
    parse_text(text, chain).map_err(|e| match e {
        ParseError::NoLines => bad("text"),
    })
}

fn aliases_of(store: &Store, chain: Option<Chain>) -> Result<HashMap<String, Alias>, StoreError> {
    chain.map_or(Ok(HashMap::new()), |c| store.aliases(c.name()))
}

fn cents(c: i64) -> i32 {
    c.clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

pub fn parse_receipt_in(
    store: &Store,
    text: &str,
    chain: Option<&str>,
) -> Result<ReceiptView, CommandError> {
    let receipt = parse(text, chain)?;
    let aliases = aliases_of(store, receipt.chain)?;
    let proposals = propose(&receipt, |k| aliases.get(k).cloned());
    let id = receipt.id();
    let lines = receipt
        .lines
        .iter()
        .enumerate()
        .map(|(i, l)| ReceiptLineView {
            index: i as u32,
            kind: match l.kind {
                LineKind::Item => ReceiptLineKind::Item,
                LineKind::Deposit => ReceiptLineKind::Deposit,
                LineKind::DepositReturn => ReceiptLineKind::DepositReturn,
                LineKind::Discount => ReceiptLineKind::Discount,
            },
            text: l.text.clone(),
            count: l.count,
            weight: l.weight.map(|w| w.to_string()),
            amount_cents: cents(l.amount_cents),
            proposal: proposals
                .iter()
                .find(|p| p.line == i)
                .map(|p| ProposalView {
                    item: p.alias.as_ref().map(|a| a.item.clone()),
                    qty: p.qty.map(|q| q.to_string()),
                    location: p.alias.as_ref().map(|a| a.location.as_str().into()),
                    price_cents: cents(p.price_cents),
                    known: p.alias.is_some(),
                }),
        })
        .collect();
    Ok(ReceiptView {
        already_imported: store.has_receipt(&id)?,
        id,
        store: receipt.chain.map(|c| c.name().into()),
        date: receipt.date.map(|d| d.to_string()),
        total_cents: receipt.total_cents.map(cents),
        lines_cents: cents(receipt.lines_cents()),
        total_ok: receipt.check_total().is_ok(),
        lines,
    })
}

pub fn import_receipt_in(
    store: &mut Store,
    input: &ReceiptImportInput,
    today: NaiveDate,
    now: DateTime<Utc>,
) -> Result<ImportResultView, CommandError> {
    let receipt = parse(&input.text, input.store.as_deref())?;
    if receipt.check_total().is_err() {
        return Err(bad("total"));
    }
    let chain = receipt.chain.ok_or_else(|| bad("store"))?;
    let id = receipt.id();
    if store.has_receipt(&id)? {
        return Err(CommandError::AlreadyImported);
    }
    let mut confirmed = Vec::new();
    for l in &input.lines {
        let n = l.line;
        let item = l.item.trim();
        if item.is_empty() {
            return Err(bad(&format!("item:{n}")));
        }
        let qty: Qty = l.qty.parse().map_err(|_| bad(&format!("qty:{n}")))?;
        let location: Location = l
            .location
            .parse()
            .map_err(|_| bad(&format!("location:{n}")))?;
        let best_before = match l.best_before.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(s) => Some(parse_date(s, today).ok_or_else(|| bad(&format!("bestBefore:{n}")))?),
        };
        confirmed.push(Confirmed {
            line: n as usize,
            item: item.into(),
            qty,
            location,
            best_before,
        });
    }
    let import = to_events(&receipt, &confirmed, now).map_err(|e| match e {
        ImportError::NoChain => bad("store"),
        ImportError::BadLine(n) | ImportError::Duplicate(n) => bad(&format!("line:{n}")),
    })?;
    let record = ReceiptRecord {
        id,
        store: chain.name().into(),
        date: receipt.date,
        total_cents: receipt.total_cents,
        text: input.text.clone(),
    };
    store
        .import_receipt(&record, &import.events, &import.learned, now)
        .map_err(|e| match e {
            StoreError::DuplicateReceipt(_) => CommandError::AlreadyImported,
            StoreError::Ledger(_) => bad("lines"),
            other => other.into(),
        })?;
    Ok(ImportResultView {
        added: import.events.len() as u32,
        learned: import.learned.len() as u32,
    })
}

#[tauri::command]
#[specta::specta]
pub fn parse_receipt(
    state: State<'_, AppState>,
    text: String,
    store: Option<String>,
) -> Result<ReceiptView, CommandError> {
    let s = state.store()?;
    parse_receipt_in(&s, &text, store.as_deref())
}

#[tauri::command]
#[specta::specta]
pub fn import_receipt(
    state: State<'_, AppState>,
    input: ReceiptImportInput,
) -> Result<ImportResultView, CommandError> {
    let mut s = state.store()?;
    import_receipt_in(&mut s, &input, Local::now().date_naive(), Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;

    const REWE: &str = include_str!("../../crates/receipt/tests/fixtures/rewe.txt");

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 8).unwrap()
    }

    fn input(lines: Vec<ReceiptLineInput>) -> ReceiptImportInput {
        ReceiptImportInput {
            text: REWE.into(),
            store: None,
            lines,
        }
    }

    fn milk(line: u32) -> ReceiptLineInput {
        ReceiptLineInput {
            line,
            item: "milk".into(),
            qty: "1l".into(),
            location: "fridge".into(),
            best_before: Some("+5".into()),
        }
    }

    #[test]
    fn parses_a_fixture_into_a_view() {
        let store = Store::open_in_memory().unwrap();
        let v = parse_receipt_in(&store, REWE, None).unwrap();
        assert_eq!(v.store.as_deref(), Some("REWE"));
        assert_eq!(v.total_cents, Some(977));
        assert!(v.total_ok && !v.already_imported);
        assert_eq!(v.lines_cents, 977);
        let milk = &v.lines[0];
        assert_eq!(milk.kind, ReceiptLineKind::Item);
        let p = milk.proposal.as_ref().unwrap();
        assert!(!p.known && p.item.is_none());
        assert_eq!(p.qty.as_deref(), Some("1"));
        assert!(v
            .lines
            .iter()
            .any(|l| l.kind == ReceiptLineKind::Discount && l.proposal.is_none()));
        assert!(v.lines.iter().any(|l| l.weight.is_some()));
    }

    #[test]
    fn unknown_store_and_empty_text_are_reported() {
        let store = Store::open_in_memory().unwrap();
        let v = parse_receipt_in(&store, "BROT 1,00\nSUMME 1,00", None).unwrap();
        assert_eq!(v.store, None);
        assert!(matches!(
            parse_receipt_in(&store, "", None),
            Err(CommandError::BadInput(f)) if f == "text"
        ));
        assert!(matches!(
            parse_receipt_in(&store, REWE, Some("aldi")),
            Err(CommandError::BadInput(f)) if f == "store"
        ));
    }

    #[test]
    fn imports_a_line_learns_it_and_refuses_a_second_import() {
        let mut store = Store::open_in_memory().unwrap();
        let now = Utc::now();
        let r = import_receipt_in(&mut store, &input(vec![milk(0)]), today(), now).unwrap();
        assert_eq!((r.added, r.learned), (1, 1));
        assert_eq!(store.inventory().in_stock().count(), 1);
        let again = import_receipt_in(&mut store, &input(vec![milk(0)]), today(), now);
        assert!(matches!(again, Err(CommandError::AlreadyImported)));
        // The learned alias now shows on the next parse, and the receipt is known.
        let v = parse_receipt_in(&store, REWE, None).unwrap();
        assert!(v.already_imported);
        let p = v.lines[0].proposal.as_ref().unwrap();
        assert!(p.known);
        assert_eq!(p.item.as_deref(), Some("milk"));
        assert_eq!(p.location.as_deref(), Some("fridge"));
    }

    #[test]
    fn bad_fields_name_the_field_and_line() {
        let mut store = Store::open_in_memory().unwrap();
        let now = Utc::now();
        let err = |mut l: ReceiptLineInput, f: fn(&mut ReceiptLineInput)| {
            f(&mut l);
            import_receipt_in(
                &mut Store::open_in_memory().unwrap(),
                &input(vec![l]),
                today(),
                now,
            )
            .unwrap_err()
        };
        let field = |e: CommandError| match e {
            CommandError::BadInput(f) => f,
            other => panic!("{other:?}"),
        };
        assert_eq!(field(err(milk(0), |l| l.qty = "lots".into())), "qty:0");
        assert_eq!(
            field(err(milk(0), |l| l.location = "attic".into())),
            "location:0"
        );
        assert_eq!(field(err(milk(0), |l| l.item = " ".into())), "item:0");
        assert_eq!(
            field(err(milk(0), |l| l.best_before = Some("x".into()))),
            "bestBefore:0"
        );
        // Line 4 is the deposit: not an item.
        assert_eq!(field(err(milk(4), |_| {})), "line:4");
        assert_eq!(store.event_count(), 0);
        // A receipt whose lines don't add up is refused.
        let broken = REWE.replace("9,77", "9,78");
        let bad_total = ReceiptImportInput {
            text: broken,
            store: None,
            lines: vec![],
        };
        let e = import_receipt_in(&mut store, &bad_total, today(), now).unwrap_err();
        assert_eq!(field(e), "total");
    }

    #[test]
    fn text_files_pass_through_and_junk_is_unreadable() {
        let v = read_receipt_file_in(REWE.as_bytes()).unwrap();
        assert_eq!(
            (v.method, v.read_by.as_str()),
            (ReceiptReadMethod::Text, "this Mac")
        );
        assert!(v.text.contains("REWE"));
        assert!(matches!(
            read_receipt_file_in(&[0, 159, 146, 150, 0]),
            Err(CommandError::ReceiptUnreadable(r)) if r == "unknown_format"
        ));
    }

    #[test]
    fn a_pdf_with_a_text_layer_reads_exactly() {
        let pdf = include_bytes!("../../crates/extract/tests/fixtures/rewe-text.pdf");
        let v = read_receipt_file_in(pdf).unwrap();
        assert_eq!(v.method, ReceiptReadMethod::PdfText);
        let store = Store::open_in_memory().unwrap();
        assert_eq!(
            parse_receipt_in(&store, &v.text, None)
                .unwrap()
                .store
                .as_deref(),
            Some("REWE")
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn a_photo_is_read_by_ocr() {
        let png = include_bytes!("../../crates/extract/tests/fixtures/rewe-clean.png");
        let v = read_receipt_file_in(png).unwrap();
        assert_eq!(v.method, ReceiptReadMethod::Ocr);
        assert!(v.text.contains("REWE"));
    }
}
