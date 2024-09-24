use crisola::peer::new_peer;

use std::{thread, time::Duration};

fn main() -> std::io::Result<()> {
    thread::scope(|scope| {
        let addr = "226.26.26.26:2626";
        let (mut peer_manger, handle) = new_peer(addr.parse().unwrap(), scope).unwrap();
        // for _ in 1..10 {
        peer_manger
            .subscribe(1212u16, |cid, data| {
                let msg = std::str::from_utf8(data).unwrap();
                println!("Got {} from  {}", &msg, &cid);
            })
            .unwrap();
        std::thread::sleep(Duration::from_secs(4));

        // peer_manger.unsubscribe(1212u16).unwrap();
        // std::thread::sleep(Duration::from_secs(3));
        // }
        println!("Done");
        // println!("Sending shutdown request");
        // peer_manger.shutdown().unwrap();
        // std::thread::sleep(Duration::from_millis(40));
        // println!("Shutdown request sent");
        if let Err(e) = handle.join() {
            println!("Crash with {:?}", e);
        }
    });
    println!("Gone for good");
    Ok(())
}
