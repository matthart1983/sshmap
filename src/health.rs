use crate::host::{Host, HostStatus};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

pub fn check_all(hosts: Arc<Mutex<Vec<Host>>>) {
    let count = {
        let h = hosts.lock().unwrap();
        // Mark all as checking
        h.len()
    };

    {
        let mut h = hosts.lock().unwrap();
        for host in h.iter_mut() {
            host.status = HostStatus::Checking;
        }
    }

    for i in 0..count {
        let hosts = Arc::clone(&hosts);
        thread::spawn(move || {
            let hostname = {
                let h = hosts.lock().unwrap();
                h[i].hostname.clone()
            };

            let status = ping_host(&hostname);

            let mut h = hosts.lock().unwrap();
            h[i].status = status;
        });
    }
}

pub fn check_one(hosts: Arc<Mutex<Vec<Host>>>, index: usize) {
    let hosts = Arc::clone(&hosts);
    thread::spawn(move || {
        let hostname = {
            let mut h = hosts.lock().unwrap();
            if index >= h.len() {
                return;
            }
            h[index].status = HostStatus::Checking;
            h[index].hostname.clone()
        };

        let status = ping_host(&hostname);

        let mut h = hosts.lock().unwrap();
        if index < h.len() {
            h[index].status = status;
        }
    });
}

fn ping_host(hostname: &str) -> HostStatus {
    let start = Instant::now();
    let output = Command::new("ping")
        .args(["-c", "1", "-W", "2", hostname])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let rtt = start.elapsed().as_secs_f64() * 1000.0;
            // Try to parse actual RTT from ping output
            let stdout = String::from_utf8_lossy(&o.stdout);
            let parsed_rtt = parse_ping_rtt(&stdout).unwrap_or(rtt);
            HostStatus::Up(parsed_rtt)
        }
        _ => HostStatus::Down,
    }
}

fn parse_ping_rtt(output: &str) -> Option<f64> {
    // macOS: round-trip min/avg/max/stddev = 1.234/2.345/3.456/0.123 ms
    // Linux: rtt min/avg/max/mdev = 1.234/2.345/3.456/0.123 ms
    for line in output.lines() {
        if line.contains("avg") && line.contains('/') {
            let parts: Vec<&str> = line.split('=').collect();
            if let Some(vals) = parts.last() {
                let nums: Vec<&str> = vals.trim().split('/').collect();
                if nums.len() >= 2 {
                    return nums[1].trim().parse().ok();
                }
            }
        }
    }
    None
}
