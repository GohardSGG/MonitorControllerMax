desc:SUB 监听控制
//tags: Monitor Control Post
//author: GoHardSGG & AI Assistant

//---------------------- 独立的 SUB 控制滑块 ----------------------
slider1:100<0,100,1>SUB F Level
slider2:100<0,100,1>SUB B Level
slider3:100<0,100,1>SUB L Level
slider4:100<0,100,1>SUB R Level

slider99:50<0,100,1>SUB Group Volume
slider100:0<0,1,1{Off,On}>DIM SUB Group
slider101:0<0,1,1{Off,On}>Mute SUB Group
slider102:0<0,1,1{Off,On}>SUB Group Boost

//---------------------- 链接的 Mute 控制滑块 (只读取) ----------------------
slider11:0<0,1,1{Off,On}>Mute_L
slider12:0<0,1,1{Off,On}>Mute_R
slider13:0<0,1,1{Off,On}>Mute_C
slider14:0<0,1,1{Off,On}>Mute_LFE
slider15:0<0,1,1{Off,On}>Mute_LSS
slider16:0<0,1,1{Off,On}>Mute_RSS
slider17:0<0,1,1{Off,On}>Mute_LRS
slider18:0<0,1,1{Off,On}>Mute_RRS
slider19:0<0,1,1{Off,On}>Mute_LTF
slider20:0<0,1,1{Off,On}>Mute_RTF
slider21:0<0,1,1{Off,On}>Mute_LTB
slider22:0<0,1,1{Off,On}>Mute_RTB
slider23:0<0,1,1{Off,On}>Mute_LBF
slider24:0<0,1,1{Off,On}>Mute_RBF
slider25:0<0,1,1{Off,On}>Mute_LBB
slider26:0<0,1,1{Off,On}>Mute_RBB
slider27:0<0,1,1{Off,On}>Mute_SUB_F
slider28:0<0,1,1{Off,On}>Mute_SUB_B
slider29:0<0,1,1{Off,On}>Mute_SUB_L
slider30:0<0,1,1{Off,On}>Mute_SUB_R

// Solo滑块slider31-50在此脚本中不需要定义，因为音频处理只依赖Mute滑块

//---------------------- 输入输出定义 (全部 20 个) ---------------------
in_pin:Input L
in_pin:Input R
in_pin:Input C
in_pin:Input LFE
in_pin:Input LSS
in_pin:Input RSS
in_pin:Input LRS
in_pin:Input RRS
in_pin:Input LTF
in_pin:Input RTF
in_pin:Input LTB
in_pin:Input RTB
in_pin:Input LBF
in_pin:Input RBF
in_pin:Input LBB
in_pin:Input RBB
in_pin:Input SUB F
in_pin:Input SUB B
in_pin:Input SUB L
in_pin:Input SUB R

out_pin:Output L
out_pin:Output R
out_pin:Output C
out_pin:Output LFE
out_pin:Output LSS
out_pin:Output RSS
out_pin:Output LRS
out_pin:Output RRS
out_pin:Output LTF
out_pin:Output RTF
out_pin:Output LTB
out_pin:Output RTB
out_pin:Output LBF
out_pin:Output RBF
out_pin:Output LBB
out_pin:Output RBB
out_pin:Output SUB F
out_pin:Output SUB B
out_pin:Output SUB L
out_pin:Output SUB R

//---------------------- 初始化 ---------------------------
@init
scale = 0.01;

//---------------------- 滑动条处理 -----------------------
@slider
// 计算 SUB 通道的独立控制参数
Level_SUB_F_Ind    = slider1  * scale;
Level_SUB_B_Ind    = slider2  * scale;
Level_SUB_L_Ind    = slider3  * scale;
Level_SUB_R_Ind    = slider4  * scale;

Dim_SUB_Group      = slider100;
Mute_SUB_Group_Master = slider101;
Level_SUB_Group_Master = (slider99 * scale) * (Dim_SUB_Group ? 3.162 : 1) * (Mute_SUB_Group_Master ? 0 : 1);
Boost_SUB_Group    = slider102 ? 1.5 : 1; // 假设 Boost 是 2倍 (+6dB)，如果需要+10dB，请使用 3.162

//---------------------- 音频处理 -------------------------
@sample

// 非 SUB 通道 (spl0-spl15): 只受链接过来的 Mute 滑块控制
spl0  *= (1 - slider11);
spl1  *= (1 - slider12);
spl2  *= (1 - slider13);
spl3  *= (1 - slider14);
spl4  *= (1 - slider15);
spl5  *= (1 - slider16);
spl6  *= (1 - slider17);
spl7  *= (1 - slider18);
spl8  *= (1 - slider19);
spl9  *= (1 - slider20);
spl10 *= (1 - slider21);
spl11 *= (1 - slider22);
spl12 *= (1 - slider23);
spl13 *= (1 - slider24);
spl14 *= (1 - slider25);
spl15 *= (1 - slider26);

// SUB 通道 (spl16-spl19): 
// 1. 受链接过来的 Mute 滑块控制 (来自全局Solo逻辑)
// 2. 再应用本脚本内的独立 SUB 控制
Gain_Global_Mute_SUB_F = 1 - slider27;
Gain_Global_Mute_SUB_B = 1 - slider28;
Gain_Global_Mute_SUB_L = 1 - slider29;
Gain_Global_Mute_SUB_R = 1 - slider30;

spl16 *= Gain_Global_Mute_SUB_F;
spl16 *= Level_SUB_F_Ind;
spl16 *= Boost_SUB_Group;
spl16 *= Level_SUB_Group_Master;

spl17 *= Gain_Global_Mute_SUB_B;
spl17 *= Level_SUB_B_Ind;
spl17 *= Boost_SUB_Group;
spl17 *= Level_SUB_Group_Master;

spl18 *= Gain_Global_Mute_SUB_L;
spl18 *= Level_SUB_L_Ind;
spl18 *= Boost_SUB_Group;
spl18 *= Level_SUB_Group_Master;

spl19 *= Gain_Global_Mute_SUB_R;
spl19 *= Level_SUB_R_Ind;
spl19 *= Boost_SUB_Group;
spl19 *= Level_SUB_Group_Master;
