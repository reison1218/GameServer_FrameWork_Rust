use crate::battle::battle::BattleData;
use crate::battle::battle_enum::buff_type::{
    ADD_ATTACK_AND_AOE, PAIR_SAME_ELEMENT_ADD_ATTACK, RESET_MAP_ADD_ATTACK,
};
use crate::battle::battle_enum::{BattleCterState, SkillConsumeType};
use crate::battle::battle_enum::{TargetType, TRIGGER_SCOPE_NEAR_TEMP_ID};
use crate::battle::battle_skill::Skill;
use crate::battle::battle_trigger::TriggerEvent;
use crate::room::character::BattleCharacter;
use crate::room::map_data::TileMap;
use crate::TEMPLATES;
use log::{error, warn};
use protobuf::Message;
use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::ops::Deref;
use std::str::FromStr;
use tools::cmd_code::ClientCode;
use tools::protos::base::{ActionUnitPt, SummaryDataPt};
use tools::protos::battle::S_SUMMARY_NOTICE;
use tools::protos::server_protocol::R_G_SUMMARY;

impl BattleData {
    ///处理战斗结算，不管地图刷新逻辑
    /// 返回一个元组类型：是否结算，存活玩家数量，第一名的玩家列表
    pub unsafe fn battle_summary(&mut self) -> Option<R_G_SUMMARY> {
        let allive_count = self
            .battle_cter
            .values()
            .filter(|x| x.state == BattleCterState::Alive)
            .count();
        let battle_members_ptr =
            self.battle_cter.borrow_mut() as *mut HashMap<u32, BattleCharacter>;
        let battle_cters = battle_members_ptr.as_mut().unwrap();
        //如果达到结算条件，则进行结算
        if allive_count <= 1 {
            let mut member: Option<u32> = None;
            for member_cter in self.battle_cter.values() {
                if member_cter.is_died() {
                    continue;
                }
                member = Some(member_cter.user_id);
            }
            if let Some(member) = member {
                self.rank_vec.push(vec![member]);
            }
            //等级
            let mut grade;
            let mut ssn = S_SUMMARY_NOTICE::new();
            let mut rank = 0_u32;

            let mut index = self.rank_vec.len();
            if index == 0 {
                return None;
            } else {
                index -= 1;
            }
            let mut max_grade = 2_i32;
            let max_grade_temp = TEMPLATES.get_constant_ref().temps.get("max_grade");
            match max_grade_temp {
                Some(max_grade_temp) => {
                    if u32::from_str(max_grade_temp.value.as_str()).is_ok() {
                        let res = u32::from_str(max_grade_temp.value.as_str()).unwrap();
                        max_grade = res as i32;
                    }
                }
                None => {
                    error!("max_grade is not find!");
                }
            }
            let mut rgs = R_G_SUMMARY::new();
            loop {
                for members in self.rank_vec.get(index) {
                    if members.is_empty() {
                        continue;
                    }
                    for member_id in members.iter() {
                        let cter = battle_cters.get_mut(member_id);
                        if cter.is_none() {
                            error!("handler_summary!cter is not find!user_id:{}", member_id);
                            continue;
                        }
                        let cter = cter.unwrap();
                        grade = cter.grade as i32;

                        //处理grade升级和降级
                        if rank == 0 {
                            grade += 1;
                        } else {
                            grade -= 1;
                        }
                        //满足条件就初始化
                        if grade > max_grade {
                            grade = max_grade;
                        }
                        if grade <= 0 {
                            grade = 1;
                        }
                        let mut smp = SummaryDataPt::new();
                        smp.user_id = *member_id;
                        smp.cter_id = cter.cter_id;
                        smp.rank = rank;
                        smp.grade = grade as u32;
                        ssn.summary_datas.push(smp.clone());
                        rgs.summary_datas.push(smp);
                    }
                    rank += 1;
                }
                if index > 0 {
                    index -= 1;
                } else {
                    break;
                }
            }

            let res = ssn.write_to_bytes();

            match res {
                Ok(bytes) => {
                    let v = self.get_battle_cters_vec();
                    for member_id in v {
                        self.send_2_client(ClientCode::SummaryNotice, member_id, bytes.clone());
                    }
                }
                Err(e) => {
                    error!("{:?}", e);
                }
            }
            return Some(rgs);
        }
        None
    }

    ///使用道具,道具都是一次性的，用完了就删掉
    /// user_id:使用道具的玩家
    /// item_id:道具id
    pub fn use_item(
        &mut self,
        user_id: u32,
        item_id: u32,
        targets: Vec<u32>,
        au: &mut ActionUnitPt,
    ) -> anyhow::Result<Option<Vec<ActionUnitPt>>> {
        let battle_cter = self.get_battle_cter(Some(user_id));
        if let Err(e) = battle_cter {
            error!("{:?}", e);
            anyhow::bail!("")
        }
        let battle_cter = battle_cter.unwrap();
        let item = battle_cter.items.get(&item_id);
        if let None = item {
            error!(
                "item is None!user_id:{},item_id:{}",
                battle_cter.user_id, item_id
            );
            anyhow::bail!("")
        }
        let item = item.unwrap();
        let skill_id = item.skill_temp.id;
        let res = self.use_skill(user_id, skill_id, true, targets, au)?;
        let battle_cter = self.get_battle_cter_mut(Some(user_id)).unwrap();
        //用完了就删除
        battle_cter.items.remove(&item_id);
        Ok(res)
    }

    ///跳过回合
    pub fn skip_turn(&mut self, _au: &mut ActionUnitPt) {
        //直接下一个turn
        self.next_turn();
    }

    ///使用技能
    /// user_id:使用技能的玩家id
    /// target_array目标数组
    pub fn use_skill(
        &mut self,
        user_id: u32,
        skill_id: u32,
        is_item: bool,
        target_array: Vec<u32>,
        au: &mut ActionUnitPt,
    ) -> anyhow::Result<Option<Vec<ActionUnitPt>>> {
        let mut au_vec: Option<Vec<ActionUnitPt>> = None;
        unsafe {
            //战斗角色
            let battle_cter_ptr =
                self.get_battle_cter_mut(Some(user_id)).unwrap() as *mut BattleCharacter;
            let battle_cter = battle_cter_ptr.as_mut().unwrap();
            //战斗角色身上的技能
            let mut skill_s;
            let skill;
            if is_item {
                let res = TEMPLATES.get_skill_ref().get_temp(&skill_id);
                if let Err(e) = res {
                    error!("{:?}", e);
                    anyhow::bail!("")
                }
                let skill_temp = res.unwrap();
                skill_s = Some(Skill::from(skill_temp));
                skill = skill_s.as_mut().unwrap();
            } else {
                skill = battle_cter.skills.get_mut(&skill_id).unwrap();
                let res = self.before_use_skill_trigger(user_id);
                if let Err(e) = res {
                    warn!("{:?}", e);
                    anyhow::bail!("")
                }
            }

            let target = skill.skill_temp.target;
            let target_type = TargetType::try_from(target as u8).unwrap();

            //技能判定
            let skill_judge = skill.skill_temp.skill_judge as u32;

            //校验目标类型
            let res = self.check_target_array(user_id, target_type, &target_array);
            if let Err(e) = res {
                let str = format!("{:?}", e);
                warn!("{:?}", str);
                anyhow::bail!("")
            }

            //校验技能可用判定条件
            if skill_judge > 0 {
                self.check_skill_judge(user_id, skill_judge, Some(skill_id), None)?;
            }

            //根据技能id去找函数指针里面的函数，然后进行执行
            let self_ptr = self as *mut BattleData;
            for skill_ids in self_ptr.as_ref().unwrap().skill_cmd_map.keys() {
                if !skill_ids.deref().contains(&skill_id) {
                    continue;
                }
                let fn_ptr = self.skill_cmd_map.get_mut(skill_ids.deref()).unwrap();
                au_vec = fn_ptr(self, user_id, skill_id, target_array, au);
                break;
            }

            //如果不是用能量的，则重制cd
            if skill.skill_temp.consume_type != SkillConsumeType::Energy as u8 {
                skill.reset_cd();
            } else if skill.skill_temp.consume_value > battle_cter.energy {
                battle_cter.energy = 0;
            } else {
                battle_cter.energy -= skill.skill_temp.consume_value;
            }
            //使用技能后触发
            self.after_use_skill_trigger(user_id, skill_id, is_item, au);
        }
        Ok(au_vec)
    }

    ///普通攻击
    /// user_id:发动普通攻击的玩家
    /// targets:被攻击目标
    pub unsafe fn attack(
        &mut self,
        user_id: u32,
        targets: Vec<u32>,
        au: &mut ActionUnitPt,
    ) -> anyhow::Result<()> {
        let battle_cters = &mut self.battle_cter as *mut HashMap<u32, BattleCharacter>;
        let cter = battle_cters.as_mut().unwrap().get_mut(&user_id).unwrap();
        let mut aoe_buff: Option<u32> = None;

        //塞选出ape的buff
        cter.buffs
            .values()
            .filter(|buff| ADD_ATTACK_AND_AOE.contains(&buff.id))
            .for_each(|buff| {
                aoe_buff = Some(buff.id);
            });

        let index = targets.get(0).unwrap();
        let target_cter = self.get_battle_cter_mut_by_cell_index(*index as usize);

        if let Err(e) = target_cter {
            warn!("{:?}", e);
            anyhow::bail!("")
        }

        let target_cter = target_cter.unwrap();
        let target_user_id = target_cter.user_id;
        let target_user_index = target_cter.get_cell_index();
        if target_user_id == user_id {
            let str = format!("the attack target can not be Self!user_id:{}", user_id);
            warn!("{:?}", str);
            anyhow::bail!("")
        }
        if target_cter.is_died() {
            let str = format!("the target is died!user_id:{}", target_cter.user_id);
            warn!("{:?}", str);
            anyhow::bail!("")
        }

        //扣血
        let target_pt = self.deduct_hp(user_id, target_user_id, None, true);

        if let Err(e) = target_pt {
            error!("{:?}", e);
            anyhow::bail!("")
        }
        let mut target_pt = target_pt.unwrap();
        //目标被攻击，触发目标buff
        if target_pt.effects.get(0).unwrap().effect_value > 0 {
            self.attacked_buffs_trigger(target_user_id, &mut target_pt);
        }
        au.targets.push(target_pt);
        //检查aoebuff
        if let Some(buff) = aoe_buff {
            let buff = TEMPLATES.get_buff_ref().get_temp(&buff);
            if let Err(e) = buff {
                warn!("{:?}", e);
                anyhow::bail!("")
            }
            let scope_temp = TEMPLATES
                .get_skill_scope_ref()
                .get_temp(&TRIGGER_SCOPE_NEAR_TEMP_ID);
            if let Err(e) = scope_temp {
                warn!("{:?}", e);
                anyhow::bail!("")
            }
            let scope_temp = scope_temp.unwrap();
            let (_, v) = self.cal_scope(
                user_id,
                target_user_index as isize,
                TargetType::OtherAnyPlayer,
                None,
                Some(scope_temp),
            );

            //目标周围的玩家
            for target_user in v {
                if target_user_id == target_user {
                    continue;
                }
                //扣血
                let target_pt = self.deduct_hp(user_id, target_user, None, false);
                match target_pt {
                    Ok(mut target_pt) => {
                        //目标被攻击，触发目标buff
                        self.attacked_buffs_trigger(target_user, &mut target_pt);
                        au.targets.push(target_pt);
                    }
                    Err(e) => error!("{:?}", e),
                }
            }
        }
        cter.is_can_attack = false;
        Ok(())
    }

    ///刷新地图
    pub fn reset_map(&mut self, is_world_cell: Option<bool>) -> anyhow::Result<()> {
        //地图刷新前触发buff
        self.before_map_refresh_buff_trigger();
        let allive_count = self
            .battle_cter
            .values()
            .filter(|x| x.state == BattleCterState::Alive)
            .count();
        let res = TileMap::init(allive_count as u8, is_world_cell)?;

        self.tile_map = res;
        self.reflash_map_turn = Some(self.next_turn_index);
        unsafe {
            //刷新角色状态和触发地图刷新的触发buff
            for cter in self.battle_cter.values_mut() {
                if cter.is_died() {
                    continue;
                }
                cter.round_reset();
                let cter = cter as *mut BattleCharacter;
                for buff in cter.as_mut().unwrap().buffs.values_mut() {
                    //刷新地图增加攻击力
                    if RESET_MAP_ADD_ATTACK.contains(&buff.id) {
                        cter.as_mut().unwrap().trigger_add_damage_buff(buff.id);
                    }
                    //匹配相同元素的地图块加攻击，在地图刷新的时候，攻击要减回来
                    if PAIR_SAME_ELEMENT_ADD_ATTACK.contains(&buff.id) {
                        cter.as_mut().unwrap().add_damage_buffs.remove(&buff.id);
                    }
                }
            }
        }
        Ok(())
    }
    ///翻地图块
    pub fn open_cell(
        &mut self,
        index: usize,
        au: &mut ActionUnitPt,
    ) -> anyhow::Result<Option<Vec<ActionUnitPt>>> {
        let user_id = self.get_turn_user(None);
        if let Err(e) = user_id {
            warn!("{:?}", e);
            anyhow::bail!("open_cell fail!")
        }
        let user_id = user_id.unwrap();
        let str = format!(
            "open_cell fail!user_id:{},index:{}",
            user_id, self.next_turn_index
        );
        let is_pair;
        unsafe {
            let battle_cters = &mut self.battle_cter as *mut HashMap<u32, BattleCharacter>;
            let battle_cter = battle_cters.as_mut().unwrap().get_mut(&user_id);
            if let None = battle_cter {
                error!("battle_cter is not find!user_id:{}", user_id);
                anyhow::bail!("{:?}", str.as_str())
            }
            let battle_cter = battle_cter.unwrap();

            //先移动
            let v = self.handler_cter_move(user_id, index, au);
            if let Err(e) = v {
                warn!("{:?}", e);
                anyhow::bail!("{:?}", str.as_str())
            }
            let v = v.unwrap();
            //判断玩家死了没
            if battle_cter.is_died() {
                return Ok(Some(v));
            }
            //再配对
            is_pair = self.handler_cell_pair(user_id);

            //处理翻地图块触发buff
            let res = self.open_cell_buff_trigger(user_id, au, is_pair);
            if let Err(e) = res {
                anyhow::bail!("{:?}", e)
            }

            //更新翻的地图块下标
            battle_cter.open_cell_vec.push(index);
            //翻块次数-1
            battle_cter.residue_open_times -= 1;

            //玩家技能cd-1
            battle_cter.skills.values_mut().for_each(|skill| {
                if !skill.is_active {
                    skill.sub_cd(None)
                }
            });
            Ok(Some(v))
        }
    }

    ///下个turn
    pub fn next_turn(&mut self) {
        //计算下一个回合
        self.add_next_turn_index();
        //给客户端推送战斗turn推送
        self.send_battle_turn_notice();
        //创建战斗turn定时器任务
        self.build_battle_turn_task();
    }

    ///回合开始触发
    pub fn turn_start_summary(&mut self) {
        let user_id = self.get_turn_user(None);
        if let Err(e) = user_id {
            error!("{:?}", e);
            return;
        }
        let battle_data = self as *mut BattleData;
        let user_id = user_id.unwrap();
        let battle_cter = self.get_battle_cter_mut(Some(user_id));
        if let Ok(battle_cter) = battle_cter {
            if !battle_cter.is_died() {
                //结算玩家自己的状态
                battle_cter.turn_reset();
            }
        }

        //结算玩家身上的buff
        for cter in self.battle_cter.values_mut() {
            for buff in cter.buffs.clone().values() {
                let buff_id = buff.id;
                unsafe {
                    battle_data.as_mut().unwrap().consume_buff(
                        buff_id,
                        Some(cter.user_id),
                        None,
                        true,
                    );
                }
            }
        }

        //结算该玩家加在地图块上的buff
        for cell in self.tile_map.map.iter_mut() {
            for buff_id in cell.buffs.clone().keys() {
                let buff_id = *buff_id;
                unsafe {
                    battle_data.as_mut().unwrap().consume_buff(
                        buff_id,
                        None,
                        Some(cell.index),
                        true,
                    );
                }
            }
        }
    }
}
