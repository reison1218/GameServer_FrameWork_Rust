use crate::net::{new_server_tcp, Forward};
use crate::Lock;
use async_trait::async_trait;
use log::error;
use tools::tcp::TcpSender;
use tools::util::packet::Packet;

#[derive(Clone)]
pub struct BattleTcpServerHandler {
    pub token: usize,
    pub gm: Lock,
}

unsafe impl Send for BattleTcpServerHandler {}

unsafe impl Sync for BattleTcpServerHandler {}

impl Forward for BattleTcpServerHandler {
    fn get_battle_token(&self) -> Option<usize> {
        Some(self.token)
    }

    fn get_gate_token(&self) -> Option<usize> {
        None
    }

    fn get_game_center_mut(&mut self) -> &mut Lock {
        &mut self.gm
    }
}

#[async_trait]
impl tools::tcp::Handler for BattleTcpServerHandler {
    async fn try_clone(&self) -> Self {
        self.clone()
    }

    ///客户端tcp链接激活事件
    async fn on_open(&mut self, sender: TcpSender) {
        self.token = sender.token;
        self.gm.lock().await.add_battle_client(sender);
    }

    ///客户端tcp链接关闭事件
    async fn on_close(&mut self) {
        let token = self.token;
        self.gm.lock().await.battle_clients.remove(&token);
    }

    ///客户端读取事件
    async fn on_message(&mut self, mess: Vec<u8>) -> bool {
        let packet_array = Packet::build_array_from_server(mess);

        if let Err(e) = packet_array {
            error!("{:?}", e);
            return true;
        }
        let packet_array = packet_array.unwrap();

        self.forward_packet(packet_array).await;
        true
    }
}

///创建新的tcp服务器,如果有问题，终端进程
pub fn new(address: String, rm: Lock) {
    let sh = BattleTcpServerHandler { token: 0, gm: rm };
    let m = new_server_tcp(address, sh);
    async_std::task::spawn(m);
}
