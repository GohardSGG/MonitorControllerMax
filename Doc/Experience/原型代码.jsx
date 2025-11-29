import React, { useState, useEffect, useRef } from 'react';
import { Volume2, Power, Mic, Activity, Trash2, Settings, Minimize2, X, ChevronDown } from 'lucide-react';

// --- Components ---

// 1. Tech/Industrial Volume Knob
const VolumeKnob = ({ value, onChange, min = 0, max = 100 }) => {
  const [isDragging, setIsDragging] = useState(false);
  const knobRef = useRef(null);

  const angle = (value / max) * 270 - 135;

  const handleMouseDown = (e) => {
    setIsDragging(true);
  };

  useEffect(() => {
    const handleMouseMove = (e) => {
      if (!isDragging || !knobRef.current) return;
      const sensitivity = 0.5;
      const deltaY = e.movementY; 
      
      let newValue = value - deltaY * sensitivity; 
      if (newValue < min) newValue = min;
      if (newValue > max) newValue = max;
      
      onChange(Math.round(newValue));
    };

    const handleMouseUp = () => {
      setIsDragging(false);
    };

    if (isDragging) {
      window.addEventListener('mousemove', handleMouseMove);
      window.addEventListener('mouseup', handleMouseUp);
    }
    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  }, [isDragging, value, onChange, min, max]);

  // SVG Calculations
  const radius = 38; // Slightly larger for cleaner look
  const circumference = 2 * Math.PI * radius;
  const arcLength = circumference * 0.75;
  const strokeDashoffset = arcLength - (value / max) * arcLength;

  return (
    <div className="flex flex-col items-center justify-center space-y-2">
      <div 
        ref={knobRef}
        onMouseDown={handleMouseDown}
        className="relative w-24 h-24 flex items-center justify-center cursor-ns-resize group"
      >
        {/* Background Track - Thin, dark, precise */}
        <svg className="absolute w-full h-full transform rotate-[135deg]" viewBox="0 0 100 100">
           {/* Tick marks ring (optional decoration for tech feel) */}
           <circle
            cx="50" cy="50" r="46"
            fill="none"
            stroke="#cbd5e1"
            strokeWidth="1"
            strokeDasharray="2 4"
            className="opacity-50"
           />
           
           <circle
            cx="50" cy="50" r={radius}
            fill="none"
            stroke="#e2e8f0" 
            strokeWidth="4"
            strokeLinecap="butt" // Hard edges
            strokeDasharray={`${arcLength} ${circumference}`}
           />
           {/* Active Value Ring - High contrast black or dark grey */}
           <circle
            cx="50" cy="50" r={radius}
            fill="none"
            stroke={value > 90 ? '#ef4444' : '#334155'} // Red warning or Dark Slate
            strokeWidth="4"
            strokeLinecap="butt"
            strokeDasharray={`${arcLength} ${circumference}`}
            strokeDashoffset={strokeDashoffset}
            className="transition-all duration-75"
           />
        </svg>

        {/* The Knob Handle - Flat, minimal */}
        <div 
          className="w-16 h-16 bg-white border-2 border-slate-400 flex items-center justify-center transform transition-transform duration-75 shadow-sm hover:border-slate-600"
          style={{ transform: `rotate(${angle}deg)` }}
        >
          {/* Indicator Line */}
          <div className="w-1 h-6 bg-slate-800 absolute top-1 rounded-none"></div>
        </div>
      </div>
      
      {/* Digital Display - Sharp box */}
      <div className="bg-white border border-slate-400 px-2 py-0.5 text-slate-900 font-mono text-xs font-bold w-16 text-center shadow-[1px_1px_0px_rgba(0,0,0,0.1)]">
        {value.toFixed(1)}%
      </div>
    </div>
  );
};

// 2. Brutalist Control Button
const ControlButton = ({ label, active, onClick, danger = false, size = "md", fullWidth = false }) => {
  // Hard edges, solid borders, no glow
  const baseClasses = "transition-all duration-100 font-bold border border-slate-400 focus:outline-none active:translate-y-[1px] active:shadow-none select-none";
  
  // Inactive: White bg, Slate text
  // Active: Black bg/White text OR Yellow bg/Black text (Industrial look)
  
  let colorClasses = "";
  if (active) {
    if (danger) {
      // Danger Active: Red background, white text, hard border
      colorClasses = "bg-red-600 text-white border-red-700 shadow-[inset_0_2px_4px_rgba(0,0,0,0.2)]";
    } else {
      // Normal Active: Yellow/Black (like the 'Safe Mode' in reference) or Slate/White
      colorClasses = "bg-yellow-300 text-slate-900 border-slate-500 shadow-[inset_0_1px_2px_rgba(0,0,0,0.1)]";
    }
  } else {
    // Inactive
    colorClasses = "bg-white text-slate-600 hover:bg-slate-50 hover:text-slate-900 shadow-[1px_1px_0px_rgba(0,0,0,0.1)]";
  }

  const sizeClasses = size === "lg" ? "h-14 text-sm" : "h-10 text-xs";
  const widthClass = fullWidth ? "w-full" : "w-20";

  return (
    <button onClick={onClick} className={`${baseClasses} ${colorClasses} ${sizeClasses} ${widthClass} flex items-center justify-center uppercase tracking-wide`}>
      {label}
    </button>
  );
};

// 3. Technical Speaker Box
const Speaker = ({ name, id, isActive, onToggle }) => {
  return (
    <div 
      onClick={() => onToggle(id)}
      className={`
        cursor-pointer transition-all duration-150 flex items-center justify-center relative
        border hover:border-slate-800
        ${name.includes("SUB") || name === "LFE" ? 'w-20 h-20' : 'w-24 h-24'} 
        ${isActive 
          ? 'bg-slate-800 text-white border-slate-900 shadow-[2px_2px_0px_rgba(0,0,0,0.2)]' 
          : 'bg-white border-slate-300 text-slate-400 hover:text-slate-600 hover:shadow-[2px_2px_0px_rgba(0,0,0,0.1)]'}
      `}
    >
      {/* Corner accents for tech feel */}
      <div className={`absolute top-0 left-0 w-1 h-1 ${isActive ? 'bg-white' : 'bg-slate-300'}`}></div>
      <div className={`absolute top-0 right-0 w-1 h-1 ${isActive ? 'bg-white' : 'bg-slate-300'}`}></div>
      <div className={`absolute bottom-0 left-0 w-1 h-1 ${isActive ? 'bg-white' : 'bg-slate-300'}`}></div>
      <div className={`absolute bottom-0 right-0 w-1 h-1 ${isActive ? 'bg-white' : 'bg-slate-300'}`}></div>

      <span className="font-bold text-sm tracking-wider font-mono">{name}</span>
    </div>
  );
};

// --- Main App Component ---

const MonitorController = () => {
  const [volume, setVolume] = useState(8.0);
  const [controls, setControls] = useState({
    solo: false,
    dim: false,
    mute: false,
    masterMute: false,
    effect: false
  });
  
  const [activeSpeakers, setActiveSpeakers] = useState(['L', 'C', 'R', 'LFE', 'LR', 'RR']);
  const [role, setRole] = useState('Standalone');
  const [format, setFormat] = useState('5.1');
  const [logs, setLogs] = useState([]);

  const toggleControl = (key) => {
    setControls(prev => {
      const newState = { ...prev, [key]: !prev[key] };
      addLog(`[ACTION] ${key.toUpperCase()} turned ${newState[key] ? 'ON' : 'OFF'}`);
      return newState;
    });
  };

  const toggleSpeaker = (id) => {
    setActiveSpeakers(prev => {
      const isActive = prev.includes(id);
      const newSet = isActive ? prev.filter(s => s !== id) : [...prev, id];
      addLog(`[ROUTING] Speaker ${id} ${isActive ? 'muted' : 'active'}`);
      return newSet;
    });
  };

  const addLog = (msg) => {
    const time = new Date().toLocaleTimeString([], { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit' });
    setLogs(prev => [`[${time}] ${msg}`, ...prev].slice(0, 10));
  };

  const clearLogs = () => setLogs([]);

  return (
    <div className="min-h-screen bg-[#e5e7eb] text-slate-800 font-sans flex items-center justify-center p-4 md:p-8">
      
      {/* Main Window Container - Sharp, Boxy */}
      <div className="w-full max-w-5xl bg-white border-2 border-slate-500 shadow-[8px_8px_0px_rgba(0,0,0,0.15)] overflow-hidden flex flex-col h-[800px]">
        
        {/* 1. Header (Window Bar) - Minimalist */}
        <div className="h-9 bg-white border-b-2 border-slate-200 flex items-center justify-between px-3 select-none">
          <div className="flex items-center gap-4">
             <span className="text-lg font-light tracking-tighter text-slate-900">
              Monitor<span className="font-bold">Controller</span>
            </span>
            <span className="text-[10px] bg-slate-200 px-1 py-0.5 text-slate-600 font-mono">v2.0.1</span>
          </div>
          
          <div className="flex space-x-3 items-center">
            <button className="text-xs hover:underline text-slate-500">Settings</button>
            <div className="h-3 w-px bg-slate-300"></div>
            <Minimize2 size={14} className="text-slate-400 hover:text-slate-900 cursor-pointer" />
            <X size={14} className="text-slate-400 hover:text-red-600 cursor-pointer" />
          </div>
        </div>

        {/* 2. Main Content Area */}
        <div className="flex flex-1 overflow-hidden">
          
          {/* Left Sidebar (Controls) - Darker contrast sidebar */}
          <div className="w-44 bg-slate-50 border-r-2 border-slate-200 p-5 flex flex-col items-center shrink-0 z-10 relative">
            
            {/* Top Buttons Group (SOLO / MUTE) */}
            <div className="flex flex-col space-y-3 w-full items-center mb-6">
              <ControlButton 
                label="SOLO" 
                active={controls.solo} 
                onClick={() => toggleControl('solo')} 
                size="lg"
                fullWidth={true}
              />
              <ControlButton 
                label="MUTE" 
                active={controls.mute} 
                onClick={() => toggleControl('mute')} 
                danger={true}
                size="lg"
                fullWidth={true}
              />
            </div>

            <div className="w-full h-px bg-slate-200 mb-6"></div>

            {/* Volume Section */}
            <div className="flex flex-col items-center w-full space-y-4 mb-6">
              <VolumeKnob 
                value={volume} 
                onChange={(v) => setVolume(v)} 
              />
              
              {/* DIM Button - MOVED HERE */}
              <div className="w-full mt-2">
                 <ControlButton 
                  label="DIM" 
                  active={controls.dim} 
                  onClick={() => toggleControl('dim')} 
                  size="md"
                  fullWidth={true}
                />
              </div>
            </div>

            <div className="w-full h-px bg-slate-200 mb-6 mt-auto"></div>

            {/* Bottom Buttons Group */}
            <div className="flex flex-col space-y-3 w-full items-center mb-4">
              <ControlButton 
                label="M. MUTE" 
                active={controls.masterMute} 
                onClick={() => toggleControl('masterMute')} 
                danger={true}
                fullWidth={true}
              />
               <ControlButton 
                label="EFFECT" 
                active={controls.effect} 
                onClick={() => toggleControl('effect')} 
                fullWidth={true}
              />
            </div>
          </div>

          {/* Right Main Panel */}
          <div className="flex-1 flex flex-col bg-white relative">
            
            {/* Top Settings Bar - Clean lines */}
            <div className="h-14 flex items-center justify-between px-6 border-b border-slate-100 bg-white">
               {/* Left side visualizer label or breadcrumb */}
               <div className="flex items-center text-xs text-slate-400 font-mono gap-2">
                  <Activity size={14} />
                  <span>OUTPUT ROUTING MATRIX</span>
               </div>

               {/* Controls */}
              <div className="flex items-center space-x-4">
                <div className="flex items-center border border-slate-300 px-2 py-1 bg-white hover:border-slate-400">
                   <span className="text-[10px] uppercase font-bold text-slate-400 mr-2">Role</span>
                   <select 
                    value={role} onChange={(e) => setRole(e.target.value)}
                    className="bg-transparent text-slate-700 text-sm font-medium outline-none cursor-pointer appearance-none pr-4"
                    style={{backgroundImage: 'none'}}
                  >
                    <option>Standalone</option>
                    <option>Plugin</option>
                    <option>Remote</option>
                  </select>
                  <ChevronDown size={12} className="text-slate-400" />
                </div>

                <div className="flex items-center border border-slate-300 px-2 py-1 bg-white hover:border-slate-400">
                  <select 
                    value={format} onChange={(e) => setFormat(e.target.value)}
                    className="bg-transparent text-slate-700 text-sm font-medium outline-none cursor-pointer appearance-none w-16 text-center"
                  >
                    <option>5.1</option>
                    <option>7.1</option>
                    <option>Stereo</option>
                  </select>
                  <ChevronDown size={12} className="text-slate-400" />
                </div>
              </div>
            </div>

            {/* Grid Background */}
            <div className="flex-1 relative overflow-auto p-10 flex items-center justify-center">
               {/* Subtle Grid Lines for Technical Feel */}
               <div className="absolute inset-0" 
                    style={{
                      backgroundImage: 'linear-gradient(#f1f5f9 1px, transparent 1px), linear-gradient(90deg, #f1f5f9 1px, transparent 1px)', 
                      backgroundSize: '40px 40px'
                    }}>
               </div>

               {/* Center Speaker Grid */}
               <div className="relative z-10 grid grid-cols-3 gap-x-12 gap-y-16 w-full max-w-3xl mx-auto items-center justify-items-center">
                  
                  {/* Row 1 */}
                  <Speaker name="L" id="L" isActive={activeSpeakers.includes('L')} onToggle={toggleSpeaker} />
                  <Speaker name="C" id="C" isActive={activeSpeakers.includes('C')} onToggle={toggleSpeaker} />
                  <Speaker name="R" id="R" isActive={activeSpeakers.includes('R')} onToggle={toggleSpeaker} />

                  {/* Row 2 */}
                  <Speaker name="SUB L" id="SUBL" isActive={activeSpeakers.includes('SUBL')} onToggle={toggleSpeaker} />
                  <Speaker name="LFE" id="LFE" isActive={activeSpeakers.includes('LFE')} onToggle={toggleSpeaker} />
                  <Speaker name="SUB R" id="SUBR" isActive={activeSpeakers.includes('SUBR')} onToggle={toggleSpeaker} />

                  {/* Row 3 */}
                  <div className="flex flex-col items-center gap-1">
                     <Speaker name="LR" id="LR" isActive={activeSpeakers.includes('LR')} onToggle={toggleSpeaker} />
                     <span className="text-[9px] uppercase tracking-widest text-slate-400 font-bold bg-white px-1">CH 7</span>
                  </div>
                  
                  <div className="flex flex-col items-center gap-1">
                     <Speaker name="SUB" id="SUB" isActive={activeSpeakers.includes('SUB')} onToggle={toggleSpeaker} />
                     <span className="text-[9px] uppercase tracking-widest text-slate-400 font-bold bg-white px-1">AUX</span>
                  </div>

                  <div className="flex flex-col items-center gap-1">
                     <Speaker name="RR" id="RR" isActive={activeSpeakers.includes('RR')} onToggle={toggleSpeaker} />
                     <span className="text-[9px] uppercase tracking-widest text-slate-400 font-bold bg-white px-1">CH 8</span>
                  </div>

               </div>
            </div>

            {/* Bottom Debug Panel - Code Editor Style */}
            <div className="h-40 bg-[#f8fafc] border-t-2 border-slate-200 flex flex-col font-mono">
              <div className="h-7 bg-slate-100 border-b border-slate-200 px-4 flex items-center justify-between">
                <span className="text-[10px] uppercase font-bold text-slate-500 tracking-wider">
                  Event Log
                </span>
                <button 
                  onClick={clearLogs}
                  className="text-[10px] text-slate-400 hover:text-red-500 uppercase font-bold tracking-wider transition-colors"
                >
                  Clear Console
                </button>
              </div>
              <div className="flex-1 p-2 overflow-y-auto text-xs space-y-0.5">
                {logs.length === 0 && (
                  <div className="text-slate-300 pl-2 opacity-50 select-none">-- No events logged --</div>
                )}
                {logs.map((log, i) => (
                  <div key={i} className="pl-2 border-l-2 border-transparent hover:border-slate-300 hover:bg-white text-slate-600">
                    <span className="text-slate-400 mr-2 opacity-75">{log.split(']')[0]}]</span>
                    <span className={log.includes('ACTION') ? 'text-blue-600 font-bold' : 'text-slate-700'}>
                      {log.split(']').slice(1).join(']')}
                    </span>
                  </div>
                ))}
              </div>
            </div>

          </div>
        </div>
      </div>
    </div>
  );
};

export default MonitorController;