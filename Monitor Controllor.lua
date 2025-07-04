desc:7.1.4.4 监听控制
//tags: Monitor Control 7.1.4.4
//author: GoHardSGG & AI Assistant

//---------------------- 声道增益控制 ----------------------
slider1:100<0,100,1>L/R (%)
slider2:100<0,100,1>C (%)
slider3:100<0,100,1>LFE (%)
slider4:100<0,100,1>LSS/RSS (%)
slider5:100<0,100,1>LRS/RRS (%)
slider6:100<0,100,1>LTF/RTF (%)
slider7:100<0,100,1>LTB/RTB (%)
slider8:100<0,100,1>LBF/RBF (%)
slider9:100<0,100,1>LBB/RBB (%)
slider10:100<0,100,1>SUB GROUP(%)

//---------------------- 主音量 --------------------------
slider99:50<0,100,1>Master Volume (%)
slider100:0<0,1,1{Off,On}>DIM
slider101:0<0,1,1{Off,On}>Mute Master
slider102:0<0,1,1{Off,On}>LFE +10dB

//---------------------- Mute 控制 ----------------------
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

//---------------------- Solo 控制 ---------------------- 
slider31:0<0,1,1{Off,On}>Solo_L
slider32:0<0,1,1{Off,On}>Solo_R
slider33:0<0,1,1{Off,On}>Solo_C
slider34:0<0,1,1{Off,On}>Solo_LFE
slider35:0<0,1,1{Off,On}>Solo_LSS
slider36:0<0,1,1{Off,On}>Solo_RSS
slider37:0<0,1,1{Off,On}>Solo_LRS
slider38:0<0,1,1{Off,On}>Solo_RRS
slider39:0<0,1,1{Off,On}>Solo_LTF
slider40:0<0,1,1{Off,On}>Solo_RTF
slider41:0<0,1,1{Off,On}>Solo_LTB
slider42:0<0,1,1{Off,On}>Solo_RTB
slider43:0<0,1,1{Off,On}>Solo_LBF
slider44:0<0,1,1{Off,On}>Solo_RBF
slider45:0<0,1,1{Off,On}>Solo_LBB
slider46:0<0,1,1{Off,On}>Solo_RBB
slider47:0<0,1,1{Off,On}>Solo_SUB_F
slider48:0<0,1,1{Off,On}>Solo_SUB_B
slider49:0<0,1,1{Off,On}>Solo_SUB_L
slider50:0<0,1,1{Off,On}>Solo_SUB_R

//---------------------- 输入输出定义 ---------------------
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
Pre_Solo_Active = 0;

user_mute_L = 0; user_mute_R = 0; user_mute_C = 0; user_mute_LFE = 0;
user_mute_LSS = 0; user_mute_RSS = 0; user_mute_LRS = 0; user_mute_RRS = 0;
user_mute_LTF = 0; user_mute_RTF = 0; user_mute_LTB = 0; user_mute_RTB = 0;
user_mute_LBF = 0; user_mute_RBF = 0; user_mute_LBB = 0; user_mute_RBB = 0;
user_mute_SUB_F = 0; user_mute_SUB_B = 0; user_mute_SUB_L = 0; user_mute_SUB_R = 0;

//---------------------- 滑动条处理 -----------------------
@slider

Level_LR            = slider1  * scale;
Level_C             = slider2  * scale;
Level_LFE           = slider3  * scale;
Level_Side          = slider4  * scale;
Level_Rear          = slider5  * scale;
Level_TopFront      = slider6  * scale;
Level_TopBack       = slider7  * scale;
Level_BottomFront   = slider8  * scale;
Level_BottomBack    = slider9  * scale;
Level_SUB_Group     = slider10 * scale;

Dim_Master          = slider100; 
Mute_Master         = slider101;
Level_Master        = (slider99 * scale) * (Dim_Master ? 0.16 : 1) * (Mute_Master ? 0 : 1);
LFE_Boost           = slider102 ? 3.162 : 1;

Non_SUB_Solo_Active = slider31 | slider32 | slider33 | slider34 | 
                      slider35 | slider36 | slider37 | slider38 | 
                      slider39 | slider40 | slider41 | slider42 | 
                      slider43 | slider44 | slider45 | slider46;
SUB_Solo_Active = slider47 | slider48 | slider49 | slider50;
Current_Solo_Active = Non_SUB_Solo_Active | SUB_Solo_Active;

function refresh_all_mutes() (
    sliderchange(slider11); sliderchange(slider12); sliderchange(slider13); sliderchange(slider14);
    sliderchange(slider15); sliderchange(slider16); sliderchange(slider17); sliderchange(slider18);
    sliderchange(slider19); sliderchange(slider20); sliderchange(slider21); sliderchange(slider22);
    sliderchange(slider23); sliderchange(slider24); sliderchange(slider25); sliderchange(slider26);
    sliderchange(slider27); sliderchange(slider28); sliderchange(slider29); sliderchange(slider30);
);

(Current_Solo_Active != Pre_Solo_Active) ? (
    Current_Solo_Active ? (
        user_mute_L = slider11;
        user_mute_R = slider12;
        user_mute_C = slider13;
        user_mute_LFE = slider14;
        user_mute_LSS = slider15;
        user_mute_RSS = slider16;
        user_mute_LRS = slider17;
        user_mute_RRS = slider18;
        user_mute_LTF = slider19;
        user_mute_RTF = slider20;
        user_mute_LTB = slider21;
        user_mute_RTB = slider22;
        user_mute_LBF = slider23;
        user_mute_RBF = slider24;
        user_mute_LBB = slider25;
        user_mute_RBB = slider26;
        user_mute_SUB_F = slider27;
        user_mute_SUB_B = slider28;
        user_mute_SUB_L = slider29;
        user_mute_SUB_R = slider30;
    ) : (
        slider11 = user_mute_L;
        slider12 = user_mute_R;
        slider13 = user_mute_C;
        slider14 = user_mute_LFE;
        slider15 = user_mute_LSS;
        slider16 = user_mute_RSS;
        slider17 = user_mute_LRS;
        slider18 = user_mute_RRS;
        slider19 = user_mute_LTF;
        slider20 = user_mute_RTF;
        slider21 = user_mute_LTB;
        slider22 = user_mute_RTB;
        slider23 = user_mute_LBF;
        slider24 = user_mute_RBF;
        slider25 = user_mute_LBB;
        slider26 = user_mute_RBB;
        slider27 = user_mute_SUB_F;
        slider28 = user_mute_SUB_B;
        slider29 = user_mute_SUB_L;
        slider30 = user_mute_SUB_R;
        refresh_all_mutes();
    );
    Pre_Solo_Active = Current_Solo_Active;
);

// Solo 激活时的 Mute 滑块设置逻辑
Current_Solo_Active ? (
    // 非 SUB 通道的 Mute 滑块设置
    slider11 = slider31 ? 0 : 1;
    slider12 = slider32 ? 0 : 1;
    slider13 = slider33 ? 0 : 1;
    slider14 = slider34 ? 0 : 1;
    slider15 = slider35 ? 0 : 1;
    slider16 = slider36 ? 0 : 1;
    slider17 = slider37 ? 0 : 1;
    slider18 = slider38 ? 0 : 1;
    slider19 = slider39 ? 0 : 1;
    slider20 = slider40 ? 0 : 1;
    slider21 = slider41 ? 0 : 1;
    slider22 = slider42 ? 0 : 1;
    slider23 = slider43 ? 0 : 1;
    slider24 = slider44 ? 0 : 1;
    slider25 = slider45 ? 0 : 1;
    slider26 = slider46 ? 0 : 1;

    // SUB 通道的 Mute 滑块设置
    // **修改点**：只有当 SUB Solo 激活时，SUB Mute 滑块才根据其 Solo 状态变化
    // 如果只是非 SUB Solo 激活，SUB Mute 滑块保持 user_mute
    SUB_Solo_Active ? (
        slider27 = slider47 ? 0 : 1;
        slider28 = slider48 ? 0 : 1;
        slider29 = slider49 ? 0 : 1;
        slider30 = slider50 ? 0 : 1;
    ) : (
        slider27 = user_mute_SUB_F;
        slider28 = user_mute_SUB_B;
        slider29 = user_mute_SUB_L;
        slider30 = user_mute_SUB_R;
    );
    
    refresh_all_mutes();
) : ( 
    user_mute_L = slider11;
    user_mute_R = slider12;
    user_mute_C = slider13;
    user_mute_LFE = slider14;
    user_mute_LSS = slider15;
    user_mute_RSS = slider16;
    user_mute_LRS = slider17;
    user_mute_RRS = slider18;
    user_mute_LTF = slider19;
    user_mute_RTF = slider20;
    user_mute_LTB = slider21;
    user_mute_RTB = slider22;
    user_mute_LBF = slider23;
    user_mute_RBF = slider24;
    user_mute_LBB = slider25;
    user_mute_RBB = slider26;
    user_mute_SUB_F = slider27;
    user_mute_SUB_B = slider28;
    user_mute_SUB_L = slider29;
    user_mute_SUB_R = slider30;
);

//---------------------- 音频处理 -------------------------
@sample

Non_SUB_Solo_Active = slider31 | slider32 | slider33 | slider34 | 
                      slider35 | slider36 | slider37 | slider38 | 
                      slider39 | slider40 | slider41 | slider42 | 
                      slider43 | slider44 | slider45 | slider46;
SUB_Solo_Active = slider47 | slider48 | slider49 | slider50;

// 计算非 SUB 通道的增益因子
// 1. 如果是非 SUB Solo 激活 (Non_SUB_Solo_Active)，则它们遵循自己的 Solo 逻辑。
// 2. 如果只有 SUB Solo 激活 (SUB_Solo_Active && !Non_SUB_Solo_Active)，则非 SUB 通道音频**强制通过 (Gain=1)**，
//    以确保信号进入校准软件 (忽略此时其Mute滑块可能已被@slider设为1)。
// 3. 如果没有任何 Solo 激活，则它们根据自身的 Mute 滑块状态通过。
Gain_L   = (SUB_Solo_Active && !Non_SUB_Solo_Active) ? 1 : (Non_SUB_Solo_Active ? (slider31 ? 1 : 0) : (1 - slider11));
Gain_R   = (SUB_Solo_Active && !Non_SUB_Solo_Active) ? 1 : (Non_SUB_Solo_Active ? (slider32 ? 1 : 0) : (1 - slider12));
Gain_C   = (SUB_Solo_Active && !Non_SUB_Solo_Active) ? 1 : (Non_SUB_Solo_Active ? (slider33 ? 1 : 0) : (1 - slider13));
Gain_LFE = (SUB_Solo_Active && !Non_SUB_Solo_Active) ? 1 : (Non_SUB_Solo_Active ? (slider34 ? 1 : 0) : (1 - slider14));
Gain_LSS = (SUB_Solo_Active && !Non_SUB_Solo_Active) ? 1 : (Non_SUB_Solo_Active ? (slider35 ? 1 : 0) : (1 - slider15));
Gain_RSS = (SUB_Solo_Active && !Non_SUB_Solo_Active) ? 1 : (Non_SUB_Solo_Active ? (slider36 ? 1 : 0) : (1 - slider16));
Gain_LRS = (SUB_Solo_Active && !Non_SUB_Solo_Active) ? 1 : (Non_SUB_Solo_Active ? (slider37 ? 1 : 0) : (1 - slider17));
Gain_RRS = (SUB_Solo_Active && !Non_SUB_Solo_Active) ? 1 : (Non_SUB_Solo_Active ? (slider38 ? 1 : 0) : (1 - slider18));
Gain_LTF = (SUB_Solo_Active && !Non_SUB_Solo_Active) ? 1 : (Non_SUB_Solo_Active ? (slider39 ? 1 : 0) : (1 - slider19));
Gain_RTF = (SUB_Solo_Active && !Non_SUB_Solo_Active) ? 1 : (Non_SUB_Solo_Active ? (slider40 ? 1 : 0) : (1 - slider20));
Gain_LTB = (SUB_Solo_Active && !Non_SUB_Solo_Active) ? 1 : (Non_SUB_Solo_Active ? (slider41 ? 1 : 0) : (1 - slider21));
Gain_RTB = (SUB_Solo_Active && !Non_SUB_Solo_Active) ? 1 : (Non_SUB_Solo_Active ? (slider42 ? 1 : 0) : (1 - slider22));
Gain_LBF = (SUB_Solo_Active && !Non_SUB_Solo_Active) ? 1 : (Non_SUB_Solo_Active ? (slider43 ? 1 : 0) : (1 - slider23));
Gain_RBF = (SUB_Solo_Active && !Non_SUB_Solo_Active) ? 1 : (Non_SUB_Solo_Active ? (slider44 ? 1 : 0) : (1 - slider24));
Gain_LBB = (SUB_Solo_Active && !Non_SUB_Solo_Active) ? 1 : (Non_SUB_Solo_Active ? (slider45 ? 1 : 0) : (1 - slider25));
Gain_RBB = (SUB_Solo_Active && !Non_SUB_Solo_Active) ? 1 : (Non_SUB_Solo_Active ? (slider46 ? 1 : 0) : (1 - slider26));

// 应用增益
spl0  *= Level_LR        * Gain_L * Level_Master;
spl1  *= Level_LR        * Gain_R * Level_Master;
spl2  *= Level_C         * Gain_C * Level_Master;
spl3  *= Level_LFE       * Gain_LFE * LFE_Boost * Level_Master;
spl4  *= Level_Side      * Gain_LSS * Level_Master;
spl5  *= Level_Side      * Gain_RSS * Level_Master;
spl6  *= Level_Rear      * Gain_LRS * Level_Master;
spl7  *= Level_Rear      * Gain_RRS * Level_Master;
spl8  *= Level_TopFront  * Gain_LTF * Level_Master;
spl9  *= Level_TopFront  * Gain_RTF * Level_Master;
spl10 *= Level_TopBack   * Gain_LTB * Level_Master;
spl11 *= Level_TopBack   * Gain_RTB * Level_Master;
spl12 *= Level_BottomFront * Gain_LBF * Level_Master;
spl13 *= Level_BottomFront * Gain_RBF * Level_Master;
spl14 *= Level_BottomBack  * Gain_LBB * Level_Master;
spl15 *= Level_BottomBack  * Gain_RBB * Level_Master;

// SUB 通道音频在此脚本中只应用基础增益和 Master，以及它们自身的 Mute 滑块。
// 它们不受任何 Solo 逻辑的音频影响。
spl16 *= Level_SUB_Group   * (1 - slider27) * Level_Master;
spl17 *= Level_SUB_Group   * (1 - slider28) * Level_Master;
spl18 *= Level_SUB_Group   * (1 - slider29) * Level_Master;
spl19 *= Level_SUB_Group   * (1 - slider30) * Level_Master;
