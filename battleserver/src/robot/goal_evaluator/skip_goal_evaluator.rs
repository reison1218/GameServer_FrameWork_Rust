use crate::battle::{battle::BattleData, battle_player::BattlePlayer};
use crate::robot::goal_evaluator::GoalEvaluator;
use crate::robot::robot_skill::can_use_skill;
use crate::robot::robot_status::skip_action::SkipRobotAction;
use crate::robot::robot_task_mgr::RobotTask;
use crossbeam::channel::Sender;

#[derive(Default)]
pub struct SkipGoalEvaluator {
    // desirability: AtomicCell<u32>,
}

impl GoalEvaluator for SkipGoalEvaluator {
    fn calculate_desirability(&self, battle_player: &BattlePlayer) -> u32 {
        unsafe {
            let battle_data = battle_player
                .robot_data
                .as_ref()
                .unwrap()
                .battle_data
                .as_ref()
                .unwrap();
            //如果什么都干不了了，则结束turn期望值拉满
            if battle_player.flow_data.residue_movement_points == 0
                && !battle_player.is_can_attack()
                && !can_use_skill(battle_data, battle_player)
            {
                return 100;
            }
            0
        }
    }

    fn set_status(
        &self,
        robot: &BattlePlayer,
        sender: Sender<RobotTask>,
        battle_data: *mut BattleData,
    ) {
        let mut res = SkipRobotAction::new(battle_data, sender);
        res.cter_id = robot.get_cter_id();
        res.robot_id = robot.get_user_id();
        res.temp_id = robot.robot_data.as_ref().unwrap().temp_id;
        robot.change_robot_status(Box::new(res));
    }
}
