// author: Dawn
// date: 2024-04-21
// description: check system info and push alerts information to wechat.
// dependency: apt install pkg-config

extern crate log;
extern crate reqwest;
extern crate sysinfo;
extern crate urlencoding;
extern crate dotenv;

use env_logger::Env;
use log::{error, info, warn};
use sysinfo::{CpuRefreshKind, Disks, RefreshKind, System};
use tokio::time::{sleep, Duration};
use urlencoding::encode;
use dotenv::dotenv;
use std::env;

#[tokio::main]
#[cfg(not(all(feature = "color", feature = "humantime")))]
async fn main() {
    // 初始化日志
    let log_env = Env::default()
        .filter_or("MY_LOG_LEVEL", "info")
        .write_style_or("MY_LOG_STYLE", "always");

    env_logger::init_from_env(log_env);

    // 加载 .env 配置
    dotenv().ok();

    let pushplus_token = env::var("PUSHPLUS_TOKEN").expect("PUSHPLUS_TOKEN is not set, please set it in .env");

    let mut sys = System::new_all();
    loop {
        info!("Check system schedule every 10m ...");
        sys.refresh_all();

        // 内存
        let memory: u64 = sys.total_memory();
        let memory_used = sys.used_memory();
        let memory_use_percent = memory_used as f32 / memory as f32 * 100.0;
        if memory_use_percent > 70.0 {
            let content = format!(
                "Memory used over {:.2}% - alert by Dawn.",
                memory_use_percent
            );
            warn!("{}", &content);
            if let Err(err) = send_alert(&pushplus_token, &content).await {
                error!("Failed to send alert: {}", err);
            }
        }

        // 磁盘
        let disks = Disks::new_with_refreshed_list();
        for disk in disks.list() {
            let disk_usage = disk.total_space() - disk.available_space();
            let disk_use_percent = disk_usage as f32 / disk.total_space() as f32 * 100.0;
            if disk_use_percent > 80.0 {
                let content = format!(
                    "disk {:?} usage over - {:.2}% - alert by Dawn.",
                    disk.name(),
                    disk_use_percent
                );
                warn!("{}", &content);
                if let Err(err) = send_alert(&pushplus_token, &content).await {
                    error!("Failed to send alert: {}", err);
                }
            }
        }

        // CPU 负载
        let mut s =
            System::new_with_specifics(RefreshKind::new().with_cpu(CpuRefreshKind::everything()));
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
        s.refresh_cpu();
        let load_avg = System::load_average();
        if load_avg.fifteen > s.cpus().len() as f64 * 2.0 {
            let content = format!(
                "CPU load average too high: {:.2} {:.2} {:.2} - alert by Dawn.",
                load_avg.one, load_avg.five, load_avg.fifteen
            );
            warn!("{}", &content);
            if let Err(err) = send_alert(&pushplus_token, &content).await {
                error!("Failed to send alert: {}", err);
            }
        }
        // 每 10 分钟检查一次
        sleep(Duration::from_secs(600)).await;
    }
}

async fn send_alert(token: &str, content: &str) -> Result<(), reqwest::Error> {
    let os = System::name().unwrap_or("Unknown OS".to_string());
    let hostname = System::host_name().unwrap_or("Unknown Hostname".to_string());
    let full_content = format!("{} - {} - {}", os, hostname, content);
    let encode_content = encode(&full_content);
    let url = format!(
        "https://www.pushplus.plus/send?token={}&content={}",
        token,
        encode_content
    );
    let _resp = reqwest::get(&url).await?;
    Ok(())
}
