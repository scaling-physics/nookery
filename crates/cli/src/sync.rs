//! `nookery sync serve|pair|now`: the Mac hub without the app (for testing
//! the phone or an emulator: it reaches the Mac at 10.0.2.2) and the phone's
//! side from a terminal. Same settings as the app (`sync.secret`,
//! `sync.peer`), so a data folder works with either.

use std::net::UdpSocket;
use std::path::Path;
use std::sync::{Arc, Mutex};

use clap::{Args, Subcommand};
use nookery_store::Store;
use nookery_sync::{
    connect, start, sync_now, Secret, StoreProvider, DEFAULT_PORT, SETTING_SYNC_SECRET,
};

type Res<T> = Result<T, Box<dyn std::error::Error>>;

const SETTING_PEER: &str = "sync.peer";

#[derive(Args)]
pub struct SyncArgs {
    #[command(subcommand)]
    pub cmd: SyncCmd,
}

#[derive(Subcommand)]
pub enum SyncCmd {
    /// Share this data folder over the home network until stopped (Ctrl-C);
    /// prints the pairing code and addresses.
    Serve {
        /// Port to listen on (default 47480).
        #[arg(long, default_value_t = DEFAULT_PORT)]
        port: u16,
        /// Listen on this address only (default: all interfaces).
        #[arg(long, default_value = "0.0.0.0")]
        bind: String,
        /// Make a new pairing code first (phones paired before must pair again).
        #[arg(long)]
        new_code: bool,
    },
    /// Pair this data folder (acting as the phone) with a Mac.
    Pair {
        /// The code shown on the Mac, e.g. ABCD-EFGH-IJKL-MNOP-QRST-UV.
        code: String,
        /// The Mac's address, e.g. 192.168.1.20 or 10.0.2.2:47480.
        address: String,
    },
    /// Sync with the paired Mac now.
    Now,
}

fn lan_ip() -> Option<String> {
    let s = UdpSocket::bind("0.0.0.0:0").ok()?;
    s.connect("192.0.2.1:9").ok()?;
    Some(s.local_addr().ok()?.ip().to_string())
}

pub fn run(store: &mut Store, dir: &Path, args: SyncArgs) -> Res<()> {
    match args.cmd {
        SyncCmd::Serve {
            port,
            bind,
            new_code,
        } => {
            let have = store
                .setting(SETTING_SYNC_SECRET)?
                .and_then(|h| Secret::from_hex(&h));
            let secret = match have {
                Some(s) if !new_code => s,
                _ => {
                    let s = Secret::generate().map_err(|_| "no randomness available")?;
                    store.set_setting(SETTING_SYNC_SECRET, &s.to_hex())?;
                    s
                }
            };
            // The server gets its own connection to the same file.
            let shared = Arc::new(Mutex::new(Store::open_dir(dir)?));
            let server = start(
                &bind,
                port,
                Arc::new(StoreProvider {
                    store: shared,
                    device: "Mac".into(),
                }),
            )?;
            println!("Sharing {} (Ctrl-C to stop)", dir.display());
            println!("Pairing code: {}", secret.to_code());
            if let Some(ip) = lan_ip() {
                println!("Address:      {ip}:{}", server.port());
            }
            println!("Android emulator: 10.0.2.2:{}", server.port());
            loop {
                std::thread::sleep(std::time::Duration::from_secs(3600));
            }
        }
        SyncCmd::Pair { code, address } => {
            let secret = Secret::from_code(&code).ok_or("that pairing code doesn't look right")?;
            let mut peer = connect(&address, &secret)
                .map_err(|e| format!("couldn't pair with {address}: {e}"))?;
            let (name, _) = peer.hello("cli")?;
            store.set_setting(SETTING_SYNC_SECRET, &secret.to_hex())?;
            store.set_setting(SETTING_PEER, &address)?;
            println!("Paired with {name} at {address}.");
            Ok(())
        }
        SyncCmd::Now => {
            let address = store
                .setting(SETTING_PEER)?
                .filter(|a| !a.is_empty())
                .ok_or("not paired yet: nookery sync pair CODE ADDRESS")?;
            let secret = store
                .setting(SETTING_SYNC_SECRET)?
                .and_then(|h| Secret::from_hex(&h))
                .ok_or("not paired yet: nookery sync pair CODE ADDRESS")?;
            let mut peer = connect(&address, &secret)?;
            let out = sync_now(store, &mut peer)?;
            println!(
                "Synced: sent {}, kitchen now has {} events, {} notes changed.",
                out.pushed, out.pulled_events, out.docs_changed
            );
            for (uid, why) in &out.rejected {
                println!(
                    "Dropped a change ({}): {why}",
                    uid.chars().take(8).collect::<String>()
                );
            }
            Ok(())
        }
    }
}
