use crate::robot::robot_status::robot_status::RobotStatus;
use crate::robot::robot_task_mgr::RobotTask;
use crate::robot::RobotActionType;
use crossbeam::channel::Sender;
use log::error;
use serde_json::Map;
use serde_json::Value;
use tools::cmd_code::BattleCode;

///机器人状态行为trait
pub trait RobotStatusAction {
    fn set_sender(&self, sender: Sender<RobotTask>);
    fn get_cter_id(&self) -> u32;
    fn enter(&self);
    fn execute(&self);
    fn exit(&self);
    fn get_status(&self) -> RobotStatus;
    fn get_robot_id(&self) -> u32;
    fn get_sender(&self) -> &Sender<RobotTask>;
    fn send_2_battle(
        &self,
        target_index: usize,
        robot_action_type: RobotActionType,
        cmd: BattleCode,
    ) {
        let mut robot_task = RobotTask::default();
        robot_task.action_type = robot_action_type;
        let mut map = Map::new();
        map.insert("user_id".to_owned(), Value::from(self.get_robot_id()));
        map.insert("target_index".to_owned(), Value::from(target_index));
        map.insert("cmd".to_owned(), Value::from(cmd.into_u32()));
        robot_task.data = Value::from(map);
        let res = self.get_sender().send(robot_task);
        if let Err(e) = res {
            error!("{:?}", e);
        }
    }
}
