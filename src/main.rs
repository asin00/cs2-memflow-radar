/// CS2 DMA Radar - 完整版本（无调试信息）
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
    routing::get,
    Router,
};
use clap::*;
use futures::{SinkExt, StreamExt};
use log::Level;
use memflow::prelude::v1::*;
use std::net::SocketAddr;
use std::thread;
use std::time::Duration;
use tokio::sync::broadcast;
use tower_http::services::ServeDir;

// 定义偏移量结构
#[derive(Debug, Clone)]
struct Offsets {
    dw_local_player_pawn: u64,
    dw_entity_list: u64,
    dw_global_vars: u64,
    m_i_health: u64,
    m_i_pawn_armor: u64,
    m_life_state: u64,
    m_ang_eye_angles: u64,
    m_i_team_num: u64,
    m_h_player_pawn: u64,
    m_v_old_origin: u64,
    m_i_comp_teammate_color: u64,
}

impl Offsets {
    fn from_json(path: &str) -> Self {
        let content = std::fs::read_to_string(path).expect("Failed to read offsets.json");
        let json: serde_json::Value = serde_json::from_str(&content).expect("Failed to parse offsets.json");

        fn parse_hex(s: &str) -> u64 {
            u64::from_str_radix(s.trim_start_matches("0x"), 16).unwrap()
        }

        Offsets {
            dw_local_player_pawn: parse_hex(json["dwLocalPlayerPawn"].as_str().unwrap()),
            dw_entity_list: parse_hex(json["dwEntityList"].as_str().unwrap()),
            dw_global_vars: parse_hex(json["dwGlobalVars"].as_str().unwrap()),
            m_i_health: parse_hex(json["m_iHealth"].as_str().unwrap()),
            m_i_pawn_armor: parse_hex(json["m_iPawnArmor"].as_str().unwrap()),
            m_life_state: parse_hex(json["m_lifeState"].as_str().unwrap()),
            m_ang_eye_angles: parse_hex(json["m_angEyeAngles"].as_str().unwrap()),
            m_i_team_num: parse_hex(json["m_iTeamNum"].as_str().unwrap()),
            m_h_player_pawn: parse_hex(json["m_hPlayerPawn"].as_str().unwrap()),
            m_v_old_origin: parse_hex(json["m_vOldOrigin"].as_str().unwrap()),
            m_i_comp_teammate_color: parse_hex(json["m_iCompTeammateColor"].as_str().unwrap()),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
struct PlayerInfoJson {
    #[serde(skip_serializing)]
    team_id: i32,
    health: i32,
    alive: bool,
    #[serde(rename = "localPlayer")]
    local_player: bool,
    enemy: bool,
    x: f32,
    y: f32,
    z: f32,
    angles: f32,
    #[serde(rename = "sameLevel")]
    same_level: bool,
    #[serde(rename = "compTeammateColor")]
    comp_teammate_color: i32,
}

#[derive(Debug, Clone, serde::Serialize)]
struct RadarData {
    #[serde(rename = "mapName")]
    map_name: String,
    tick: u64,
    #[serde(rename = "playerList")]
    player_list: Vec<PlayerInfoJson>,
}

fn main() -> anyhow::Result<()> {
    let matches = parse_args();
    let chain = extract_args(&matches);

    // 创建 inventory + os
    let mut inventory = Inventory::scan();
    let mut os = inventory.builder().os_chain(chain).build().expect("Failed to build OS");

    println!("[*] CS2 DMA Radar with WebSocket");
    println!("[*] Looking for CS2 process...");

    // 获取进程信息
    let proc_info = loop {
        match os.process_info_by_name("cs2.exe") {
            Ok(info) => break info,
            Err(_) => {
                println!("[*] CS2 not found, waiting...");
                thread::sleep(Duration::from_secs(2));
            }
        }
    };

    let mut process = os.into_process_by_info(proc_info).expect("Failed to open process");
    println!("[+] Found CS2 process (PID: {})", process.info().pid);

    println!("[*] Loading offsets...");
    let offsets = Offsets::from_json("offsets.json");
    println!("[+] Offsets loaded");

    let client_base = process
        .module_by_name("client.dll")
        .expect("client.dll not found")
        .base;
    println!("[+] client.dll base: 0x{:x}", client_base);

    // 创建广播通道
    let (tx, _rx) = broadcast::channel(32);
    let tx_clone = tx.clone();

    // 启动 tokio runtime 用于 WebSocket 服务
    let rt = tokio::runtime::Runtime::new()?;

    // 启动数据采集线程
    let tx_radar = tx.clone();
    std::thread::spawn(move || {
        let mut process = process;
        let offsets = offsets;
        let client_base = client_base;

        loop {
            let data = collect_radar_data(&mut process, &offsets, client_base);
            let json_str = serde_json::to_string(&data).unwrap_or_else(|_| "{}".to_string());

            // 通过广播通道发送数据
            let _ = tx_radar.send(json_str);

            // 控制循环频率
            thread::sleep(Duration::from_millis(50));
        }
    });

    // 启动 WebSocket 服务
    println!("[*] Starting WebSocket server on ws://localhost:8080/radar");
    println!("[*] Open http://localhost:8080 in your browser");

    rt.block_on(server(tx_clone))
}

fn collect_radar_data(
    process: &mut (impl Process + MemoryView),
    offsets: &Offsets,
    client_base: Address,
) -> RadarData {
    let start = std::time::Instant::now();
    let null_addr = Address::NULL;
    let mut player_list = Vec::new();

    // 读取全局变量地址
    let global_vars_ptr = match process.read_addr64(client_base + offsets.dw_global_vars) {
        Ok(addr) => addr,
        Err(_) => return RadarData {
            map_name: "unknown".to_string(),
            tick: 0,
            player_list: Vec::new(),
        },
    };
    if global_vars_ptr == null_addr {
        return RadarData {
            map_name: "unknown".to_string(),
            tick: 0,
            player_list: Vec::new(),
        };
    }

    // 读取地图名
    let map_name_ptr = match process.read_addr64(global_vars_ptr + 0x188) {
        Ok(addr) => addr,
        Err(_) => return RadarData {
            map_name: "unknown".to_string(),
            tick: 0,
            player_list: Vec::new(),
        },
    };
    let map_name = if map_name_ptr != null_addr {
        let mut buf = [0u8; 64];
        let _ = process.read_raw_into(map_name_ptr, &mut buf);
        let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        String::from_utf8_lossy(&buf[..end]).to_string()
    } else {
        "unknown".to_string()
    };

    // 读取本地玩家
    let local_pawn = match process.read_addr64(client_base + offsets.dw_local_player_pawn) {
        Ok(addr) => addr,
        Err(_) => return RadarData {
            map_name: map_name.clone(),
            tick: 0,
            player_list: Vec::new(),
        },
    };
    if local_pawn == null_addr {
        return RadarData {
            map_name: map_name.clone(),
            tick: 0,
            player_list: Vec::new(),
        };
    }

    let local_team = match process.read::<i32>(local_pawn + offsets.m_i_team_num) {
        Ok(v) => v,
        Err(_) => return RadarData {
            map_name: map_name.clone(),
            tick: 0,
            player_list: Vec::new(),
        },
    };
    let local_z = match process.read::<f32>(local_pawn + offsets.m_v_old_origin + 8) {
        Ok(v) => v,
        Err(_) => return RadarData {
            map_name: map_name.clone(),
            tick: 0,
            player_list: Vec::new(),
        },
    };

    // 读取实体列表
    let entity_list_ptr = match process.read_addr64(client_base + offsets.dw_entity_list) {
        Ok(addr) => addr,
        Err(_) => return RadarData {
            map_name: map_name.clone(),
            tick: 0,
            player_list: Vec::new(),
        },
    };
    if entity_list_ptr == null_addr {
        return RadarData {
            map_name: map_name.clone(),
            tick: 0,
            player_list: Vec::new(),
        };
    }

    let entity_list = match process.read_addr64(entity_list_ptr + 0x10) {
        Ok(addr) => addr,
        Err(_) => return RadarData {
            map_name: map_name.clone(),
            tick: 0,
            player_list: Vec::new(),
        },
    };
    if entity_list == null_addr {
        return RadarData {
            map_name: map_name.clone(),
            tick: 0,
            player_list: Vec::new(),
        };
    }

    for i in 0..64 {
        let entity_addr = match process.read_addr64(entity_list + ((i + 1) * 0x70)) {
            Ok(addr) => addr,
            Err(_) => continue,
        };
        if entity_addr == null_addr {
            continue;
        }

        let pawn_handle = match process.read::<u32>(entity_addr + offsets.m_h_player_pawn) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if pawn_handle == 0 {
            continue;
        }

        let entry_idx = (pawn_handle & 0x7FFF) >> 9;
        let list_entry = match process.read_addr64(entity_list_ptr + 0x10 + 8 * entry_idx as u64) {
            Ok(addr) => addr,
            Err(_) => continue,
        };
        if list_entry == null_addr {
            continue;
        }

        let pawn_addr = match process.read_addr64(list_entry + 0x70 * (pawn_handle as u64 & 0x1FF)) {
            Ok(addr) => addr,
            Err(_) => continue,
        };
        if pawn_addr == null_addr {
            continue;
        }

        let team = match process.read::<i32>(entity_addr + offsets.m_i_team_num) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let health = match process.read::<i32>(pawn_addr + offsets.m_i_health) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let life_state = match process.read::<i32>(pawn_addr + offsets.m_life_state) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let is_alive = life_state == 256;

        if !is_alive {
            continue;
        }

        let x = match process.read::<f32>(pawn_addr + offsets.m_v_old_origin + 4) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let y = match process.read::<f32>(pawn_addr + offsets.m_v_old_origin) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let z = match process.read::<f32>(pawn_addr + offsets.m_v_old_origin + 8) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let angle = match process.read::<f32>(pawn_addr + offsets.m_ang_eye_angles + 4) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let comp_teammate_color = match process.read::<i32>(entity_addr + offsets.m_i_comp_teammate_color) {
            Ok(v) => v,
            Err(_) => -1,
        };

        let is_local = local_pawn == pawn_addr;
        let is_enemy = local_team != team;
        let is_same_level = (z - local_z).abs() < 250.0;

        player_list.push(PlayerInfoJson {
            team_id: team,
            health,
            alive: is_alive,
            local_player: is_local,
            enemy: is_enemy,
            x,
            y,
            z,
            angles: 90.0 - angle,
            same_level: is_same_level,
            comp_teammate_color,
        });
    }

    let elapsed = start.elapsed().as_millis() as u64;

    RadarData {
        map_name: map_name,
        tick: elapsed,
        player_list: player_list,
    }
}

async fn server(tx: broadcast::Sender<String>) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/radar", get(websocket_handler))
        .nest_service("/", ServeDir::new("./client/dist"))
        .with_state(tx);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("[+] Server running on http://{}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    axum::extract::State(tx): axum::extract::State<broadcast::Sender<String>>,
) -> Response {
    ws.protocols(["v12.stomp", "v11.stomp", "v10.stomp"])
        .on_upgrade(|socket| websocket_loop(socket, tx))
}

async fn websocket_loop(socket: WebSocket, tx: broadcast::Sender<String>) {
    use std::sync::Arc;
    use std::sync::Mutex;

    let (mut sender, mut receiver) = socket.split();

    // 订阅广播通道
    let mut rx = tx.subscribe();

    // 使用通道将 STOMP 响应发回主任务
    let (response_tx, mut response_rx) = tokio::sync::mpsc::unbounded_channel();

    // 用于存储订阅 ID - 使用 Arc+Mutex 共享
    let subscription_id = Arc::new(Mutex::new(String::from("sub-0")));
    let subscription_id_clone = subscription_id.clone();

    // 接收任务：处理 STOMP 协议帧
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                // 检查是否是 STOMP CONNECT 帧
                if text.contains("CONNECT") && text.contains("accept-version") {
                    // 发送 STOMP CONNECTED 帧
                    let connected_frame = "CONNECTED\nversion:1.2\n\n\0";
                    let _ = response_tx.send(connected_frame.to_string());
                }
                // 检查是否是 SUBSCRIBE 帧
                else if text.contains("SUBSCRIBE") && text.contains("/topic/radar") {
                    // 提取 subscription id
                    let mut id = String::from("sub-0");
                    for line in text.lines() {
                        if line.starts_with("id:") {
                            id = line.trim_start_matches("id:").to_string();
                            break;
                        }
                    }
                    // 更新共享的 subscription_id
                    let mut guard = subscription_id_clone.lock().unwrap();
                    *guard = id.clone();
                }
                // 忽略其他 STOMP 帧（心跳等）
            }
        }
    });

    // 发送任务：推送雷达数据和 STOMP 响应
    let send_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                // 处理 STOMP 响应
                Some(resp) = response_rx.recv() => {
                    if sender.send(Message::Text(resp)).await.is_err() {
                        break;
                    }
                }
                // 处理雷达数据
                Ok(msg) = rx.recv() => {
                    // 获取当前的 subscription_id
                    let id = {
                        let guard = subscription_id.lock().unwrap();
                        guard.clone()
                    };
                    // 构建 STOMP MESSAGE 帧
                    let stomp_frame = format!(
                        "MESSAGE\nsubscription:{}\ncontent-type:application/json\n\n{}\0",
                        id,
                        msg
                    );
                    if sender.send(Message::Text(stomp_frame)).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    tokio::pin!(send_task);
    tokio::pin!(recv_task);

    tokio::select! {
        _ = &mut send_task => {
            recv_task.abort();
        }
        _ = &mut recv_task => {
            send_task.abort();
        }
    }
}

fn parse_args() -> ArgMatches {
    Command::new("cs2_radar")
        .version("0.1.0")
        .author("memflow")
        .arg(Arg::new("verbose").short('v').action(ArgAction::Count))
        .arg(
            Arg::new("connector")
                .long("connector")
                .short('c')
                .action(ArgAction::Append)
                .required(false),
        )
        .arg(
            Arg::new("os")
                .long("os")
                .short('o')
                .action(ArgAction::Append)
                .required(true),
        )
        .get_matches()
}

fn extract_args(matches: &ArgMatches) -> OsChain<'_> {
    let log_level = match matches.get_count("verbose") {
        0 => Level::Error,
        1 => Level::Warn,
        2 => Level::Info,
        3 => Level::Debug,
        4 => Level::Trace,
        _ => Level::Trace,
    };
    let _ = simplelog::TermLogger::init(
        log_level.to_level_filter(),
        simplelog::Config::default(),
        simplelog::TerminalMode::Stdout,
        simplelog::ColorChoice::Auto,
    );

    let conn_iter = matches
        .indices_of("connector")
        .zip(matches.get_many::<String>("connector"))
        .map(|(a, b)| a.zip(b.map(String::as_str)))
        .into_iter()
        .flatten();

    let os_iter = matches
        .indices_of("os")
        .zip(matches.get_many::<String>("os"))
        .map(|(a, b)| a.zip(b.map(String::as_str)))
        .into_iter()
        .flatten();

    OsChain::new(conn_iter, os_iter).expect("Failed to create OsChain")
}