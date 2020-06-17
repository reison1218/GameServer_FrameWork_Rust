///游戏服务专用命令号段枚举
pub enum GameCode{
    //最小值
    Min = 1000,
    //心跳
    HeartBeat = 1001,
    //离线
    LineOff = 1002,
    //登陆
    Login = 1003,
    //同步数据
    SyncData = 1004,
    //请求修改昵称
    ModifyNickName = 1005,
    //创建房间
    CreateRoom = 1006,
    //加入房间
    JoinRoom = 1007,
    //匹配房间
    SearchRoom = 1008,
    //范围最大值
    Max = 10000,
}

///房间服专属命令号段枚举
pub enum RoomCode{
    //范围最小值
    Min = 20001,
    //离线
    LineOff = 20002,
    //创建房间
    CreateRoom = 20003,
    //离开房间
    LeaveRoom = 20004,
    //T人
    Kick = 20005,
    //换队伍
    ChangeTeam = 20006,
    //准备与取消
    PrepareCancel = 20007,
    //开始游戏
    StartGame = 20008,
    //加入房间
    JoinRoom = 20009,
    //匹配房间
    SearchRoom = 20010,
    //房间设置
    RoomSetting = 20011,
    //选择角色和技能
    ChoiceCharacter = 20012,
    //表情符号
    Emoji = 20013,
    //返回最大值
    Max = 30000,
}

///客户端专属命令号段枚举
pub enum ClientCode{
    //最小命令号
    Min = 10001,
    //返回心跳包
    HeartBeat = 10002,
    //返回登陆命令号
    Login = 10003,
    //返回同步命令号
    SyncData = 10004,
    //请求修改昵称返回
    NickNameModify = 10005,
    //房间命令号
    Room = 10006,
    //离开房间命令号
    LeaveRoom = 10007,
    //开始游戏
    Start = 10008,
    //换队伍
    ChangeTeam = 10009,
    //准备与取消准备
    PrepareCancel=10010,
    //房间设置
    RoomSetting=10011,
    //房间成员变动推送消息
    RoomMemberNotice=10012,
    //T人返回
    KickMember=10013,
    //选择角色
    ChoiceCharacter=10014,
    //房间推送
    RoomNotice=10015,
    //表情符号
    Emoji=10016,
    //最大命令号
    Max = 20000,
}