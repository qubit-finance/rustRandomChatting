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

use futures_channel::mpsc::{unbounded, UnboundedSender}; //, UnboundedReceiver};
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};
use tokio;
use tokio::net::{TcpListener, TcpStream};
use tungstenite::protocol::Message;
//use tokio_tungstenite::WebSocketStream;

type Tx = UnboundedSender<Message>;
//type Rx = UnboundedReceiver<Message>;
// type TxMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;
// type RxMap = Arc<Mutex<HashMap<SocketAddr, Rx>>>;
// type NameMap = Arc<Mutex<HashMap<SocketAddr, String>>>;
// type IsInMap = Arc<Mutex<HashMap<SocketAddr, bool>>>;
//type WS = WebSocketStream<TcpStream>;
// type PeerMap = Arc<Mutex<HashMap<SocketAddr, (Tx, Rx, SocketAddr)>>>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;
type PeerVecMap = Arc<Mutex<HashMap<SocketAddr, SocketAddr>>>;
type ActiveClinetDeque = Arc<Mutex<VecDeque<SocketAddr>>>;
//type IsInMutex = Arc<Mutex<bool>>;

async fn handle_connection(peer_map: PeerMap, peer_vec_map: PeerVecMap, active_client_deque: ActiveClinetDeque, raw_stream: TcpStream, addr: SocketAddr){
    println!("Incoming TCP connection from: {}", addr);

    // 문제 생겨도 panic하지 않고 죽도록 그냥 리턴함.
    let ws_stream = match tokio_tungstenite::accept_async(raw_stream).await{
        Ok(ws_stream) => ws_stream,
        Err(_) => return,
    };
        // .expect("Error during the websocket handshake occurred");
    println!("WebSocket connection established: {}", addr);
    let (outgoing, incoming) = ws_stream.split();
    // Insert the write part of this peer to the peer map.
    let (tx, rx) = unbounded();

    peer_map.lock().unwrap().insert(addr, tx.clone());
    //let my_mutex = IsInMutex::new(Mutex::new(false));

    active_client_deque.lock().unwrap().push_back(addr);
    // let mut cnt = 0;
    let peer_addr: SocketAddr;
    let mut success = false;
    
    // loading msg 띄워주기
    for _ in 1..=101{
        // 남이 나를 고른 경우
        if peer_vec_map.lock().unwrap().contains_key(&addr) {
            
            // 받는거 처리
            let peer_addr = peer_vec_map.lock().unwrap().get(&addr).unwrap().clone();
            let send_to_peer = incoming.try_for_each(|msg| {
                println!("Received a message from {}: {}", addr, msg.to_text().unwrap());
                    // send msg
                peer_map.lock().unwrap().get(&peer_addr).unwrap().unbounded_send(msg.clone()).unwrap();            
                future::ok(())
            });
            let receive_from_peer = rx.map(Ok).forward(outgoing);
            pin_mut!(send_to_peer, receive_from_peer);
            future::select(send_to_peer, receive_from_peer).await;
            // 마무리
            peer_vec_map.lock().unwrap().remove(&peer_addr);
            break;
        }

        if active_client_deque.lock().unwrap().len() <= 1 {
            tokio::time::sleep(Duration::from_millis(5000)).await;
        }
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

            // if let Some(one_addr) = active_client_deque.lock().unwrap().get(1) {
            //     if one_addr == &addr {
            //         peer_addr = active_client_deque.lock().unwrap().pop_front().unwrap();
            //         active_client_deque.lock().unwrap().pop_front().unwrap();
            //     }
            // }

                
            if !success {tokio::time::sleep(Duration::from_millis(500)).await; continue;}
                
            peer_addr = *peer_vec_map.lock().unwrap().get(&addr).unwrap();
            let send_to_peer = incoming.try_for_each(|msg| {
                println!("Received a message from {}: {}", addr, msg.to_text().unwrap());
                    // send msg
                peer_map.lock().unwrap().get(&peer_addr).unwrap().unbounded_send(msg.clone()).unwrap();            
                future::ok(())
            });
            let receive_from_peer = rx.map(Ok).forward(outgoing);
            pin_mut!(send_to_peer, receive_from_peer);
            future::select(send_to_peer, receive_from_peer).await;
            // 마무리
            peer_vec_map.lock().unwrap().remove(&peer_addr);
            break;
                
        }
        
    }
    if !success {
        println!("Connection Failed");
        return;
    }
    // while active_client_deque.lock().unwrap().len() <= 1 || active_client_deque.lock().unwrap().get(1).unwrap() != &addr{
        
    //     cnt += 1;
    //     if cnt >= 100 {
    //         // 나중에 여기 다르게 처리
    //         println!("Connection Failed");
    //         break;
    //     }
    // }
    // {

    // }
    
    

    //let tx = tx.clone();
    // loop{
    //     // check mutex

    //     if peer_vec_map.lock().unwrap().contains_key(&addr) {
            
    //         // 받는거 처리
        
    //         continue;
    //     }
    //     // get msg
    //     let msg = incoming.try_next().await.unwrap().unwrap();

    //     // is start
    //     if msg.to_string() == "/s".to_string() || msg.to_string() == "/start".to_string(){
    //         // check activeClient
    //         active_client_deque.lock().unwrap().push_back(addr);
    //         let mut cnt = 0;
            
    //         // loading msg
    //         while active_client_deque.lock().unwrap().len() <= 1 {
    //             tokio::time::sleep(Duration::from_millis(500));
    //             cnt += 1;
    //             if cnt >= 10 {
    //                 break;
    //             }
    //         }
    //         if active_client_deque.lock().unwrap().len() >= 2 {
    //             //*my_mutex.lock().unwrap() = true;
    //             let mut peer_addr = active_client_deque.lock().unwrap().pop_front().unwrap();
    //             peer_vec_map.lock().unwrap().insert(peer_addr, addr);

                

    //             let receive_from_peer = rx.map(Ok).forward(outgoing);

    //             pin_mut!(send_to_peer, receive_from_peer);
    //             future::select(send_to_peer, receive_from_peer).await;

    //             // 마무리
    //             peer_vec_map.lock().unwrap().remove(&peer_addr);
                


                
    //         };

    //     }            


    // };

    peer_map.lock().unwrap().remove(&addr);
    
}


// async fn handle_chat(peer_map:PeerMap, active_client_deque:ActiveClinetDeque){
//     let mut client1 = active_client_deque.lock().unwrap().pop_front().unwrap();
//     let mut client2 = active_client_deque.lock().unwrap().pop_front().unwrap();
// }

#[tokio::main]
async fn main() -> Result<(), IoError> {
    // address를 cli로 받고 없으면 로컬 12345
    let addr = env::args().nth(1).unwrap_or_else(|| "127.0.0.1:12345".to_string());

    let state = PeerMap::new(Mutex::new(HashMap::new()));
    let peer_vec_map = PeerVecMap::new(Mutex::new(HashMap::new()));
    let active_client_deque = ActiveClinetDeque::new(Mutex::new(VecDeque::new()));
    // let rxMap = RxMap::new(Mutex::new(HashMap::new()));
    // let nameMap = NameMap::new(Mutex::new(HashMap::new()));
    // let isinMap = IsInMap::new(Mutex::new(HashMap::new()));

    // 최초의 TCP bind
    let try_socket = TcpListener::bind(&addr).await;
    let listner = try_socket.expect("Failed to bind");
    println!("Listening on: {}", addr);

    while let Ok((stream, addr)) = listner.accept().await {
        tokio::spawn(handle_connection(state.clone(), peer_vec_map.clone(), active_client_deque.clone(), stream, addr));
    }

    Ok(())

}