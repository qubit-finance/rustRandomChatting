//! You can test this out by running:
//!
//!     cargo run --example server 127.0.0.1:12345
//! 


use std::{
    collections::{HashMap, VecDeque},
    env,
    io::Error as IoError,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
    
};

use futures_channel::mpsc::{unbounded, UnboundedSender, UnboundedReceiver};
use futures_util::{future, pin_mut, stream::{TryStreamExt, SplitStream, SplitSink}, StreamExt};
use tokio;
use tokio::net::{TcpListener, TcpStream};
use tungstenite::protocol::Message;
use tokio_tungstenite::{WebSocketStream};

type Tx = UnboundedSender<Message>;
type Rx = UnboundedReceiver<Message>;
type WS = WebSocketStream<TcpStream>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;
type PeerVecMap = Arc<Mutex<HashMap<SocketAddr, SocketAddr>>>;
type ActiveClinetDeque = Arc<Mutex<VecDeque<SocketAddr>>>;


//global constants
const TIMEOUTMSG: &str = "$T$I$M$E$O$U$T!!^^";
const STARTMSG: &str = "$S$T$A$R$T!!^^";
const SLEEPTIME: u64 = 100;

async fn handle_connection(peer_map: PeerMap, peer_vec_map: PeerVecMap, active_client_deque: ActiveClinetDeque, raw_stream: TcpStream, addr: SocketAddr){
    
    println!("Incoming TCP connection from: {}", addr);

    // 문제 생겨도 panic하지 않고 죽도록 그냥 리턴함.
    let ws_stream = match tokio_tungstenite::accept_async(raw_stream).await{
        Ok(ws_stream) => ws_stream,
        Err(_) => return,
    };

    // 성공 로그
    println!("WebSocket connection established: {}", addr);

    // 스트림 분리
    let (outgoing, incoming) = ws_stream.split();

    //sender
    let (tx, rx) = unbounded();

    // map에 나 넣기
    peer_map.lock().unwrap().insert(addr, tx.clone());

    // active에 나 추가
    active_client_deque.lock().unwrap().push_back(addr);

    //let peer_addr: SocketAddr;
    let mut success = false;
    
    // loading msg 띄워주기
    for cnt in 1..=101{

        // 남이 나를 고른 경우
        if peer_vec_map.lock().unwrap().contains_key(&addr) {        
            
            handle_chat(peer_vec_map.clone(), peer_map.clone(), addr, rx, incoming, outgoing).await;
            break;
        }

        // 내가 선택되지 않았으면 다른애 선택가능한지 체크
        if active_client_deque.lock().unwrap().len() <= 1 {
            tokio::time::sleep(Duration::from_millis(SLEEPTIME)).await;
            if cnt == 101{
                handle_timeout(tx, rx, outgoing).await;
                break;
            }
        }
        // 고를 놈이 있는 경우
        else{
            {
                let mut lck = active_client_deque.lock().unwrap();
                if lck.get(1).unwrap() == &addr {
                    let _peer_addr = &lck.pop_front().unwrap().clone();
                    // peer_addr = _peer_addr.clone();
                    lck.pop_front().unwrap();
                    peer_vec_map.lock().unwrap().insert(*_peer_addr, addr);
                    peer_vec_map.lock().unwrap().insert(addr, *_peer_addr);
                    success = true;
                }
            }
           
            // lock걸고하는 중간에 실패했으면 잠자고 continue
            if !success {
                tokio::time::sleep(Duration::from_millis(SLEEPTIME)).await; 
                if cnt == 101{
                    handle_timeout(tx, rx, outgoing).await;
                    break;
                }

                continue;}
                
            handle_chat(peer_vec_map.clone(), peer_map.clone(), addr, rx, incoming, outgoing).await;
            break;
                
        }

        
    }

    // 넣어둔거 제거
    peer_map.lock().unwrap().remove(&addr);
    
}

fn send_start_msg(peer_addr: SocketAddr, peer_map: PeerMap){

    match peer_map.lock().unwrap().get(&peer_addr){
        Some(_tx) =>  _tx.unbounded_send(Message::Text(STARTMSG.to_string())).unwrap(),
        None => return,
    }

}

async fn handle_timeout(tx: Tx, rx: Rx, outgoing: SplitSink<WS, Message>){
    tx.unbounded_send(Message::Text(TIMEOUTMSG.to_string())).unwrap();
    rx.map(Ok).forward(outgoing).await.unwrap();
    println!("Connection Failed");
}

async fn handle_chat(peer_vec_map: PeerVecMap, peer_map: PeerMap, addr: SocketAddr, rx: Rx,
     incoming: SplitStream<WS>, outgoing: SplitSink<WS, Message>){
    // peer addr 가져오기
    let peer_addr = peer_vec_map.lock().unwrap().get(&addr).unwrap().clone();
    // start msg 전송
    send_start_msg(peer_addr, peer_map.clone());
    
    let send_to_peer = incoming.try_for_each(|msg| {
        println!("Received a message from {}: {}", addr, msg.to_text().unwrap());
        // send msg
        // if msg.to_text().unwrap() == STARTMSG {
        //     return future::ok(());
        // }

        match peer_map.lock().unwrap().get(&peer_addr){
            Some(_tx) =>  _tx.unbounded_send(msg.clone()).unwrap(),
            None => return future::ok(()),
        }         
        future::ok(())
    });
    let receive_from_peer = rx.map(Ok).forward(outgoing);
    pin_mut!(send_to_peer, receive_from_peer);
    future::select(send_to_peer, receive_from_peer).await;
    // 마무리
    peer_vec_map.lock().unwrap().remove(&peer_addr);
    ()
}

#[tokio::main]
async fn main() -> Result<(), IoError> {
    // address를 cli로 받고 없으면 로컬 12345
    let addr = env::args().nth(1).unwrap_or_else(|| "127.0.0.1:12345".to_string());

    let state = PeerMap::new(Mutex::new(HashMap::new()));
    let peer_vec_map = PeerVecMap::new(Mutex::new(HashMap::new()));
    let active_client_deque = ActiveClinetDeque::new(Mutex::new(VecDeque::new()));

    // 최초의 TCP bind
    let try_socket = TcpListener::bind(&addr).await;
    let listner = try_socket.expect("Failed to bind");
    println!("Listening on: {}", addr);

    while let Ok((stream, addr)) = listner.accept().await {
        tokio::spawn(handle_connection(state.clone(), peer_vec_map.clone(), active_client_deque.clone(), stream, addr));
    }

    Ok(())

}