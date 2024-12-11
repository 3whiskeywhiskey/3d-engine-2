mod vr;

use anyhow::Result;
use env_logger;

fn main() -> Result<()> {
    env_logger::init();
    println!("Creating VR session...");
    let mut session = vr::VrSession::new()?;

    println!("Beginning frame loop - press Ctrl+C to exit");
    loop {
        session.render_frame()?;
    }
} 