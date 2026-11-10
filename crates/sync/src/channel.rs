//! The encrypted, authenticated message channel, shared by both ends.

use std::io::{Read, Write};

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::Zeroizing;

use crate::secret::Secret;
use crate::SyncError;

pub const MAGIC: &[u8; 7] = b"NKSYNC1";
pub const NONCE_LEN: usize = 24;
/// Larger than a full ledger plus framing; anything bigger is refused.
pub const MAX_FRAME: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Client,
    Server,
}

pub type Key = Zeroizing<[u8; 32]>;

/// Direction keys for one connection: (client -> server, server -> client).
pub fn session_keys(
    secret: &Secret,
    client_nonce: &[u8; NONCE_LEN],
    server_nonce: &[u8; NONCE_LEN],
) -> (Key, Key) {
    let mut salt = Vec::with_capacity(NONCE_LEN * 2);
    salt.extend_from_slice(client_nonce);
    salt.extend_from_slice(server_nonce);
    let hk = Hkdf::<Sha256>::new(Some(&salt), &secret.0);
    let mut c2s = Zeroizing::new([0u8; 32]);
    let mut s2c = Zeroizing::new([0u8; 32]);
    hk.expand(b"nookery/sync/v1/client-to-server", &mut *c2s)
        .expect("32 bytes is a valid HKDF-SHA256 output length");
    hk.expand(b"nookery/sync/v1/server-to-client", &mut *s2c)
        .expect("32 bytes is a valid HKDF-SHA256 output length");
    (c2s, s2c)
}

fn aad(role: Role, counter: u64) -> [u8; 9] {
    let mut a = [0u8; 9];
    a[0] = if role == Role::Client { b'c' } else { b's' };
    a[1..].copy_from_slice(&counter.to_le_bytes());
    a
}

/// `nonce || ciphertext`, bound to the sender's role and message counter.
pub(crate) fn seal(
    key: &Key,
    role: Role,
    counter: u64,
    plaintext: &[u8],
) -> Result<Vec<u8>, SyncError> {
    let mut nonce = [0u8; NONCE_LEN];
    getrandom::fill(&mut nonce).map_err(|_| SyncError::Io)?;
    let cipher = XChaCha20Poly1305::new((&**key).into());
    let ct = cipher
        .encrypt(
            &XNonce::try_from(&nonce[..]).map_err(|_| SyncError::Protocol)?,
            Payload {
                msg: plaintext,
                aad: &aad(role, counter),
            },
        )
        .map_err(|_| SyncError::Protocol)?;
    let mut out = Vec::with_capacity(NONCE_LEN + ct.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    Ok(out)
}

pub fn open(key: &Key, role: Role, counter: u64, sealed: &[u8]) -> Option<Zeroizing<Vec<u8>>> {
    if sealed.len() < NONCE_LEN {
        return None;
    }
    let (nonce, ct) = sealed.split_at(NONCE_LEN);
    let cipher = XChaCha20Poly1305::new((&**key).into());
    cipher
        .decrypt(
            &XNonce::try_from(nonce).ok()?,
            Payload {
                msg: ct,
                aad: &aad(role, counter),
            },
        )
        .ok()
        .map(Zeroizing::new)
}

pub fn write_frame<W: Write>(w: &mut W, bytes: &[u8]) -> Result<(), SyncError> {
    let len = u32::try_from(bytes.len()).map_err(|_| SyncError::Protocol)?;
    w.write_all(&len.to_le_bytes()).map_err(|_| SyncError::Io)?;
    w.write_all(bytes).map_err(|_| SyncError::Io)?;
    w.flush().map_err(|_| SyncError::Io)
}

pub fn read_frame<R: Read>(r: &mut R) -> Result<Vec<u8>, SyncError> {
    let mut len = [0u8; 4];
    r.read_exact(&mut len).map_err(|_| SyncError::Io)?;
    let len = u32::from_le_bytes(len) as usize;
    if len > MAX_FRAME {
        return Err(SyncError::Protocol);
    }
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf).map_err(|_| SyncError::Io)?;
    Ok(buf)
}

/// One side of an established connection.
pub struct Channel<S: Read + Write> {
    io: S,
    role: Role,
    send_key: Key,
    recv_key: Key,
    sent: u64,
    received: u64,
}

impl<S: Read + Write> Channel<S> {
    /// `c2s`/`s2c` as returned by [`session_keys`]; `already_received` is how many messages
    /// were consumed during the handshake (the hello the server authenticated with).
    pub fn new(io: S, role: Role, c2s: Key, s2c: Key, sent: u64, received: u64) -> Self {
        let (send_key, recv_key) = match role {
            Role::Client => (c2s, s2c),
            Role::Server => (s2c, c2s),
        };
        Self {
            io,
            role,
            send_key,
            recv_key,
            sent,
            received,
        }
    }

    pub fn send(&mut self, plaintext: &[u8]) -> Result<(), SyncError> {
        let sealed = seal(&self.send_key, self.role, self.sent, plaintext)?;
        self.sent += 1;
        write_frame(&mut self.io, &sealed)
    }

    pub fn recv(&mut self) -> Result<Zeroizing<Vec<u8>>, SyncError> {
        let sealed = read_frame(&mut self.io)?;
        let peer = if self.role == Role::Client {
            Role::Server
        } else {
            Role::Client
        };
        let plain =
            open(&self.recv_key, peer, self.received, &sealed).ok_or(SyncError::Protocol)?;
        self.received += 1;
        Ok(plain)
    }
}

/// Used by the client to send the first (hello) frame before a `Channel` exists.
pub fn seal_first(key: &Key, role: Role, plaintext: &[u8]) -> Result<Vec<u8>, SyncError> {
    seal(key, role, 0, plaintext)
}
