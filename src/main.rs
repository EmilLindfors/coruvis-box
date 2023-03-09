use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    http::Response,
    response::{Html, IntoResponse},
    routing::get,
    Router, Server,
};
use serde_json::json;
use sysinfo::ComponentExt;
use sysinfo::ProcessExt;
use sysinfo::{CpuExt, DiskExt, System, SystemExt};
use tokio::sync::broadcast;

//#[derive(Clone, Debug)]
//pub enum SysInfoData {
//    CPU(Vec<f32>),
//    MEM(u64, u64),
//}

#[derive(Clone, Debug)]
pub struct Prcs {
    name: String,
    mem: u64,
    cpu: f32,
    disk_read: u64,
    disk_written: u64,
    status: String,
}

impl std::fmt::Display for Prcs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "name: {}, mem: {}, cpu: {}",
            self.name, self.mem, self.cpu
        )
    }
}

impl From<Prcs> for serde_json::Value {
    fn from(p: Prcs) -> serde_json::Value {
        json!({"name": p.name, "mem": p.mem, "cpu": p.cpu, "status": p.status, "writtenBytes": p.disk_written, "readBytes": p.disk_read})
    }
}

#[derive(Clone, Debug)]
pub struct Processes(Vec<Prcs>);

impl From<Processes> for serde_json::Value {
    fn from(p: Processes) -> serde_json::Value {
        let mut res = Vec::new();
        for r in p.0 {
            res.push(serde_json::Value::from(r))
        }

        json!(res)
    }
}
impl From<Vec<Prcs>> for Processes {
    fn from(p: Vec<Prcs>) -> Processes {
        Processes(p)
    }
}

#[derive(Clone, Debug)]
pub struct SysInfo {
    cpu: Vec<f32>,
    processes: (Prcs, Processes),
    mem: (u64, u64),
    processor: (String, Option<usize>, u64),
    name: String,
    os: String,
}

impl SysInfo {
    pub fn to_msg(&self) -> String {
        let processes: serde_json::Value = self.processes.1.clone().into();
        let this: serde_json::Value = self.processes.0.clone().into();
        json!({
            "cpu": self.cpu,
            "mem": {
                "total": self.mem.0,
                "used": self.mem.1
            },
            "prc": {
                "this": this,
                "others": processes
                },
            "inf": {
                "host": self.name,
                "os": self.os,
            },
            "pss": {
                "name": self.processor.0,
                "cores": self.processor.1,
                "mhz": self.processor.2
            }
        })
        .to_string()
    }
}

fn cpu_info(sys: &System) -> (String, Option<usize>, u64) {
    let mhz = sys.global_cpu_info().frequency();

    let cpu_brand = sys.global_cpu_info().brand();
    let cores = sys.physical_core_count();
    (cpu_brand.to_string(), cores, mhz)
}

#[tokio::main]
async fn main() {
    // note: if you send more messages, upgrade the channel size
    let (tx, _) = broadcast::channel::<SysInfo>(1);

    tracing_subscriber::fmt::init();

    let app_state = AppState { tx: tx.clone() };

    let router = Router::new()
        .route("/", get(root_get))
        .route("/index.mjs", get(indexmjs_get))
        .route("/index.css", get(indexcss_get))
        .route("/realtime/cpus", get(realtime_cpus_get))
        .with_state(app_state.clone());

    // Update CPU usage in the background
    tokio::task::spawn_blocking(move || {
        let mut sys = System::new_all();
        let cpu_info = cpu_info(&sys);
        let info = sys.long_os_version().unwrap_or("Unknown OS".to_string());
        let host = sys.host_name().unwrap_or("Unknown host".to_string());
        let tm = sys.total_memory();
        //for disk in sys.disks() {
        //    println!(
        //        "{:?}: {:?}, {:?}, total: {:?}",
        //        disk.name(),
        //        disk.type_(),
        //        disk.available_space(),
        //        disk.total_space()
        //    );
        //}

        loop {
            sys.refresh_cpu();
            sys.refresh_memory();
            sys.refresh_processes();

            let v: Vec<_> = sys.cpus().iter().map(|cpu| cpu.cpu_usage()).collect();
            let mut p: Vec<Prcs> = sys
                .processes()
                .into_iter()
                .filter(|pr| pr.1.memory() > 0)
                .map(|pr| Prcs {
                    name: pr.1.name().to_string(),
                    mem: pr.1.memory(),
                    cpu: pr.1.cpu_usage(),
                    disk_read: pr.1.disk_usage().total_read_bytes,
                    disk_written: pr.1.disk_usage().total_written_bytes,
                    status: pr.1.status().to_string(),
                })
                .collect();
            p.sort_by(|a, b| b.mem.partial_cmp(&a.mem).unwrap());

            let axact = p.iter().find(|v| v.name == "axact").unwrap();

            let um = sys.used_memory();
            let _ = tx.send(SysInfo {
                cpu: v,
                processes: (axact.clone(), p.into()),
                mem: (tm, um),
                processor: cpu_info.clone(),
                name: host.to_string(),
                os: info.to_string(),
            });

            std::thread::sleep(System::MINIMUM_CPU_UPDATE_INTERVAL);
        }
    });

    let server = Server::bind(&"0.0.0.0:7032".parse().unwrap()).serve(router.into_make_service());
    let addr = server.local_addr();
    println!("Listening on {addr}");

    server.await.unwrap();
}

#[derive(Clone, Debug)]
struct AppState {
    tx: broadcast::Sender<SysInfo>,
}

#[axum::debug_handler]
async fn root_get() -> impl IntoResponse {
    let markup = tokio::fs::read_to_string("src/index.html").await.unwrap();

    Html(markup)
}

#[axum::debug_handler]
async fn indexmjs_get() -> impl IntoResponse {
    let markup = tokio::fs::read_to_string("src/index.mjs").await.unwrap();

    Response::builder()
        .header("content-type", "application/javascript;charset=utf-8")
        .body(markup)
        .unwrap()
}

#[axum::debug_handler]
async fn indexcss_get() -> impl IntoResponse {
    let markup = tokio::fs::read_to_string("src/index.css").await.unwrap();

    Response::builder()
        .header("content-type", "text/css;charset=utf-8")
        .body(markup)
        .unwrap()
}

#[axum::debug_handler]
async fn realtime_cpus_get(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|ws: WebSocket| async { realtime_cpus_stream(state, ws).await })
}

async fn realtime_cpus_stream(app_state: AppState, mut ws: WebSocket) {
    let mut rx = app_state.tx.subscribe();

    while let Ok(msg) = rx.recv().await {
        let m = msg.to_msg();
        ws.send(Message::Text(m)).await.unwrap()
    }
}
