desc:7.1.4.4 监听控制
//tags: Monitor Control 7.1.4.4
//author: GoHardSGG

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

//---------------------- 每个音箱的静音控制 ----------------------
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

//---------------------- 每个音箱的SOLO控制 ---------------------- 
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


// 初始化Solo状态
Pre_Solo_Active = 0;

NUM_CH = 20; // 20个通道
scale = 0.01;
// 每个通道的响度状态
chn_state = 0;
loop(NUM_CH,
  chn_state[0] = 0; // peak
  chn_state[1] = 0; // rms
  chn_state += 2;
);

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

Dim_Master          = slider100; // 读取DIM状态
Mute_Master         = slider101;
Level_Master        = (slider99 * scale) * (Dim_Master ? 0.16 : 1) * (Mute_Master ? 0 : 1); // 主音量计算

// 检测是否有任意Solo被激活
Current_Solo_Active = slider31 | slider32 | slider33 | slider34 | 
              slider35 | slider36 | slider37 | slider38 | 
              slider39 | slider40 | slider41 | slider42 | 
              slider43 | slider44 | slider45 | slider46 | 
              slider47 | slider48 | slider49 | slider50;

// 状态切换时保存原始Mute
(Current_Solo_Active != Pre_Solo_Active) ? (
    Current_Solo_Active ? (
        // 进入Solo模式：保存当前用户设置
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
        // 退出Solo模式：恢复原始状态
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

        // 刷新所有Mute滑块
        sliderchange(slider11);
        sliderchange(slider12);
        sliderchange(slider13);
        sliderchange(slider14);
        sliderchange(slider15);
        sliderchange(slider16);
        sliderchange(slider17);
        sliderchange(slider18);
        sliderchange(slider19);
        sliderchange(slider20);
        sliderchange(slider21);
        sliderchange(slider22);
        sliderchange(slider23);
        sliderchange(slider24);
        sliderchange(slider25);
        sliderchange(slider26);
        sliderchange(slider27);
        sliderchange(slider28);
        sliderchange(slider29);
        sliderchange(slider30);

    );
    
    Pre_Solo_Active = Current_Solo_Active;
);

// Solo激活时的处理
Current_Solo_Active ?
 (
    // 实时计算Mute状态（不修改原始滑块值）
    Mute_L   = slider31 ? 0 : 1;  // Solo开启时保留状态，否则静音
    Mute_R   = slider32 ? 0 : 1;  
    Mute_C   = slider33 ? 0 : 1;  
    Mute_LFE = slider34 ? 0 : 1;  
    Mute_LSS = slider35 ? 0 : 1;  
    Mute_RSS = slider36 ? 0 : 1;  
    Mute_LRS = slider37 ? 0 : 1;  
    Mute_RRS = slider38 ? 0 : 1;  
    Mute_LTF = slider39 ? 0 : 1;  
    Mute_RTF = slider40 ? 0 : 1;  
    Mute_LTB = slider41 ? 0 : 1;  
    Mute_RTB = slider42 ? 0 : 1;  
    Mute_LBF = slider43 ? 0 : 1;  
    Mute_RBF = slider44 ? 0 : 1;  
    Mute_LBB = slider45 ? 0 : 1;  
    Mute_RBB = slider46 ? 0 : 1;  
    Mute_SUB_F = slider47 ? 0 : 1;  
    Mute_SUB_B = slider48 ? 0 : 1;  
    Mute_SUB_L = slider49 ? 0 : 1;  
    Mute_SUB_R = slider50 ? 0 : 1;  

    slider11  = Mute_L;
    slider12  = Mute_R;
    slider13  = Mute_C;
    slider14  = Mute_LFE;
    slider15  = Mute_LSS;
    slider16  = Mute_RSS;
    slider17  = Mute_LRS;
    slider18  = Mute_RRS;
    slider19  = Mute_LTF;
    slider20  = Mute_RTF;
    slider21  = Mute_LTB;
    slider22  = Mute_RTB;
    slider23  = Mute_LBF;
    slider24  = Mute_RBF;
    slider25  = Mute_LBB;
    slider26  = Mute_RBB;
    slider27  = Mute_SUB_F;
    slider28  = Mute_SUB_B;
    slider29  = Mute_SUB_L;
    slider30  = Mute_SUB_R;


    // 刷新所有Mute滑块
    sliderchange(slider11);
    sliderchange(slider12);
    sliderchange(slider13);
    sliderchange(slider14);
    sliderchange(slider15);
    sliderchange(slider16);
    sliderchange(slider17);
    sliderchange(slider18);
    sliderchange(slider19);
    sliderchange(slider20);
    sliderchange(slider21);
    sliderchange(slider22);
    sliderchange(slider23);
    sliderchange(slider24);
    sliderchange(slider25);
    sliderchange(slider26);
    sliderchange(slider27);
    sliderchange(slider28);
    sliderchange(slider29);
    sliderchange(slider30);
) : 
(
    // Solo未激活时，恢复用户手动设置的Mute状态
    Mute_L = slider11;
    Mute_R = slider12;
    Mute_C = slider13;
    Mute_LFE = slider14;
    Mute_LSS = slider15;
    Mute_RSS = slider16;
    Mute_LRS = slider17;
    Mute_RRS = slider18;
    Mute_LTF = slider19;
    Mute_RTF = slider20;
    Mute_LTB = slider21;
    Mute_RTB = slider22;
    Mute_LBF = slider23;
    Mute_RBF = slider24;
    Mute_LBB = slider25;
    Mute_RBB = slider26;
    Mute_SUB_F = slider27;
    Mute_SUB_B = slider28;
    Mute_SUB_L = slider29;
    Mute_SUB_R = slider30;


    // 刷新所有Mute滑块
    sliderchange(slider11);
    sliderchange(slider12);
    sliderchange(slider13);
    sliderchange(slider14);
    sliderchange(slider15);
    sliderchange(slider16);
    sliderchange(slider17);
    sliderchange(slider18);
    sliderchange(slider19);
    sliderchange(slider20);
    sliderchange(slider21);
    sliderchange(slider22);
    sliderchange(slider23);
    sliderchange(slider24);
    sliderchange(slider25);
    sliderchange(slider26);
    sliderchange(slider27);
    sliderchange(slider28);
    sliderchange(slider29);
    sliderchange(slider30);
);

// 当没有Solo时实时保存用户操作
!Current_Solo_Active ? (
   
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

       // 刷新所有Mute滑块
    sliderchange(slider11);
    sliderchange(slider12);
    sliderchange(slider13);
    sliderchange(slider14);
    sliderchange(slider15);
    sliderchange(slider16);
    sliderchange(slider17);
    sliderchange(slider18);
    sliderchange(slider19);
    sliderchange(slider20);
    sliderchange(slider21);
    sliderchange(slider22);
    sliderchange(slider23);
    sliderchange(slider24);
    sliderchange(slider25);
    sliderchange(slider26);
    sliderchange(slider27);
    sliderchange(slider28);
    sliderchange(slider29);
    sliderchange(slider30);
);

@block
// 计算RMS窗口长度
win_len = (0.4 * srate)|0;
i_win_len = 1/win_len;

//---------------------- 音频处理 -------------------------
@sample

function update_meter(spl, chn) instance(peak, rms) (
  spl = abs(spl);
  peak = max(peak * 0.95, spl); // 峰值衰减
  rms = 0.95 * rms + 0.05 * (spl*spl); // RMS平滑
);

// 应用原有处理后的信号计算响度
update_meter(spl0*0.5, 0);  // L
update_meter(spl1*0.5, 1);  // R
update_meter(spl2*0.5, 2);  // C
// ... 重复处理所有20个通道...

// 应用增益链
spl0  *= Level_LR        * (Current_Solo_Active ? (slider31 ? 1 : 0) : (1 - slider11)) * Level_Master;
spl1  *= Level_LR        * (Current_Solo_Active ? (slider32 ? 1 : 0) : (1 - slider12)) * Level_Master;  // R
spl2  *= Level_C         * (Current_Solo_Active ? (slider33 ? 1 : 0) : (1 - slider13)) * Level_Master;  // C
spl3  *= Level_LFE       * (Current_Solo_Active ? (slider34 ? 1 : 0) : (1 - slider14)) * (slider102 ? 3.162 : 1) * Level_Master;  // LFE
spl4  *= Level_Side      * (Current_Solo_Active ? (slider35 ? 1 : 0) : (1 - slider15)) * Level_Master;  // LSS
spl5  *= Level_Side      * (Current_Solo_Active ? (slider36 ? 1 : 0) : (1 - slider16)) * Level_Master;  // RSS
spl6  *= Level_Rear      * (Current_Solo_Active ? (slider37 ? 1 : 0) : (1 - slider17)) * Level_Master;  // LRS
spl7  *= Level_Rear      * (Current_Solo_Active ? (slider38 ? 1 : 0) : (1 - slider18)) * Level_Master;  // RRS
spl8  *= Level_TopFront  * (Current_Solo_Active ? (slider39 ? 1 : 0) : (1 - slider19)) * Level_Master;  // LTF
spl9  *= Level_TopFront  * (Current_Solo_Active ? (slider40 ? 1 : 0) : (1 - slider20)) * Level_Master;  // RTF
spl10 *= Level_TopBack   * (Current_Solo_Active ? (slider41 ? 1 : 0) : (1 - slider21)) * Level_Master;  // LTB
spl11 *= Level_TopBack   * (Current_Solo_Active ? (slider42 ? 1 : 0) : (1 - slider22)) * Level_Master;  // RTB
spl12 *= Level_BottomFront * (Current_Solo_Active ? (slider43 ? 1 : 0) : (1 - slider23)) * Level_Master;  // LBF
spl13 *= Level_BottomFront * (Current_Solo_Active ? (slider44 ? 1 : 0) : (1 - slider24)) * Level_Master;  // RBF
spl14 *= Level_BottomBack  * (Current_Solo_Active ? (slider45 ? 1 : 0) : (1 - slider25)) * Level_Master;  // LBB
spl15 *= Level_BottomBack  * (Current_Solo_Active ? (slider46 ? 1 : 0) : (1 - slider26)) * Level_Master;  // RBB
spl16 *= Level_SUB_Group   * (Current_Solo_Active ? (slider47 ? 1 : 0) : (1 - slider27)) * Level_Master;  // SUB_F
spl17 *= Level_SUB_Group   * (Current_Solo_Active ? (slider48 ? 1 : 0) : (1 - slider28)) * Level_Master;  // SUB_B
spl18 *= Level_SUB_Group   * (Current_Solo_Active ? (slider49 ? 1 : 0) : (1 - slider29)) * Level_Master;  // SUB_L
spl19 *= Level_SUB_Group   * (Current_Solo_Active ? (slider50 ? 1 : 0) : (1 - slider30)) * Level_Master;  // SUB_R


@gfx 600 800
// 基础布局参数
cols = 4;
rows = 5;
cell_w = gfx_w / cols;
cell_h = gfx_h / rows;
bar_w = cell_w * 0.6;
text_h = gfx_texth * 1.2;

// 绘制网格背景
gfx_set(0.2,0.2,0.2);
loop(rows+1, y=cell_h*it; gfx_line(0,y,gfx_w,y););
loop(cols+1, x=cell_w*it; gfx_line(x,0,x,gfx_h););

// 绘制每个通道的VU表
chn = 0;
loop(rows,
  y = cell_h * it + text_h;
  loop(cols,
    x = cell_w * it + (cell_w - bar_w)/2;
    
    // 获取当前通道状态
    peak = chn_state[chn*2];
    rms = sqrt(chn_state[chn*2+1]);
    
    // 绘制RMS
    gfx_set(0.5,0.7,1,0.6);
    h = min(rms * cell_h, cell_h - text_h);
    gfx_rect(x, y + (cell_h - text_h - h), bar_w, h);
    
    // 绘制峰值
    gfx_set(1,1,0.5,0.8);
    peak_h = min(peak * cell_h * 0.8, cell_h - text_h);
    gfx_rect(x, y + (cell_h - text_h - peak_h), bar_w, 2);
    
    // 通道标签
    gfx_set(1,1,1);
    gfx_x = x + bar_w/2 - gfx_texth/2;
    gfx_y = y + cell_h - text_h;
    gfx_drawstr(#+chn,1|4); // 显示通道编号
    
    chn += 1;
    chn >= NUM_CH ? break;
  );
  chn >= NUM_CH ? break;
);
