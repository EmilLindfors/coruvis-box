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
use sysinfo::{CpuExt, DiskExt, System, SystemExt, NetworkExt, ProcessExt, NetworksExt};
use tokio::sync::broadcast;
use serde_json::{Value, to_string, json};

#[tokio::main]
async fn main() {
    // note: if you send more messages, upgrade the channel size
    let (tx, _) = broadcast::channel::<Value>(1);

    tracing_subscriber::fmt::init();

    let app_state = AppState { tx: tx.clone() };

    let router = Router::new()
        .route("/", get(root_get))
        .route("/index.mjs", get(indexmjs_get))
        .route("/index.css", get(indexcss_get))
        .route("/realtime/cpus", get(realtime_cpus_get))
        .with_state(app_state.clone());

    // Update usage in the background
    tokio::task::spawn_blocking(move || {
        let mut sys = System::new_all();
     
        // static system information
        let static_info = json!({
            "host": &sys.host_name().unwrap_or("Unknown host".to_string()),
            "os": &sys.long_os_version().unwrap_or("Unknown OS".to_string()),
            "cpu": {
                "name": &sys.global_cpu_info().brand(),
                "cores": &sys.physical_core_count(),
                "mhz": &sys.global_cpu_info().frequency()
            }
            });

        loop {
            
            //collect virtual core use
            sys.refresh_cpu();
            let virtual_cores: Vec<_> = sys.cpus().iter().map(|cpu| cpu.cpu_usage()).collect();
                        
            // collect Network data
            sys.refresh_networks();
            let networks: &serde_json::Value = &sys
                .networks()
                .iter()
                .map(
                    |d| 
                    json!({"name": d.0, "received": d.1.received(), "transmitted": d.1.transmitted()})
                )
                .collect();
            
            // collect disk use
            sys.refresh_disks();
            let disks: &serde_json::Value = &sys
                .disks()
                .iter()
                //.filter(|d| d.mount_point().ends_with("coruvis-box"))
                .map(|d| json!({"name": d.name(), "total": (d.total_space() / 1_000_000) as f32, "used": (d.available_space()/ 1_000_000) as f32}))
                .collect();
                
            //collect memory usage 
            sys.refresh_memory();
            let memory = json!({"total": (sys.total_memory() / 1_000_000) as i32, "used:": (sys.used_memory() / 1_000_000) as i32});
             
            // collect and sort processes and mark our process
            sys.refresh_processes();
            let processes = &mut sys
            .processes()
            .iter()
            .filter(|pr| pr.1.memory() > 0)
            .map(|pr|         
            (("is_coruvis", pr.1.name() == "coruvis"),
            ("name", pr.1.name().to_string()),
            ("mem", pr.1.memory() / 1_000_000),
            ("cpu", pr.1.cpu_usage()),
            ("disk_read", pr.1.disk_usage().total_read_bytes / 1_000_000),
            ("disk_written", pr.1.disk_usage().total_written_bytes / 1_000_000),
            ("status", pr.1.status().to_string())))
            .collect::<Vec<_>>();
            processes.sort_by(|a, b| b.2.1.partial_cmp(&a.2.1).unwrap());
            
            
            // send the info to the websocket as json
            let _ = tx.send(json!({
                "general": static_info,
                "cpu": virtual_cores,
                "prc": processes,
                "mem": memory,
                "hdd": disks[0],
                "net": networks
            }));
            
            // update every minimum cpu interval
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
    tx: broadcast::Sender<Value>,
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
           let data = to_string(&msg).unwrap();
        ws.send(Message::Text(data)).await.unwrap()
    }
}
