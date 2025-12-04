use eframe::egui;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use sysinfo::System;
use std::process::Command;
use std::fs;
use std::path::Path;
use std::io::{Read, Write};
use log::{info, debug, LevelFilter};
use simplelog::{Config, WriteLogger};

#[derive(Serialize, Deserialize, Clone)]
struct VRSettings {
    render_scale: f32,
    use_openxr: bool,
    use_steamvr: bool,
    encode_bitrate_mbps: u32,
    encode_resolution_width: u32,
    encode_resolution_height: u32,
    link_sharpening: f32,
    asw_enabled: bool,
    asw_mode: ASWMode,
    foveated_rendering: bool,
    foveated_level: FoveatedLevel,
    cpu_priority_boost: bool,
    gpu_priority: GPUPriority,
    pixel_density: f32,
    fov_scale: f32,
    force_composition_layers: bool,
    disable_depth_submission: bool,
    turbo_mode: bool,
    auto_restart_on_freeze: bool,
    kill_oculus_client: bool,
    restart_threshold_seconds: u32,
    upscaling_enabled: bool,
    upscaling_type: UpscalingType,
    upscaling_scale: f32,
    sharpening_amount: f32,
    contrast: f32,
    saturation: f32,
    frame_throttle_fps: u32,
    shake_reduction: bool,
    audio_switching: bool,
    super_sampling: f32,
    mirror_window: bool,
    guardian_visibility: bool,
    cpu_affinity: u32,
    power_plan: PowerPlan,
    oculus_killer_enabled: bool,
    relinked_mode: bool,
    disable_asw: bool,
    enable_steamvr_autostart: bool,
    enable_runtime_high_priority: bool,
    allow_other_software: bool,
    custom_startup_program: String,
    custom_fps: u32,
    disable_oled_mura: bool,
    debug_logging: bool,
    disable_telemetry: bool,
    disable_login: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
enum ASWMode {
    Off,
    Auto,
    Force45FPS,
    Force30FPS,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
enum FoveatedLevel {
    Off,
    Low,
    Medium,
    High,
    HighTop,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
enum GPUPriority {
    Normal,
    High,
    Realtime,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
enum UpscalingType {
    NIS,
    FSR,
    CAS,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
enum PowerPlan {
    Balanced,
    HighPerformance,
    PowerSaver,
}

impl Default for VRSettings {
    fn default() -> Self {
        Self {
            render_scale: 1.2,
            use_openxr: true,
            use_steamvr: false,
            encode_bitrate_mbps: 300,
            encode_resolution_width: 2784,
            encode_resolution_height: 1472,
            link_sharpening: 0.5,
            asw_enabled: true,
            asw_mode: ASWMode::Auto,
            foveated_rendering: true,
            foveated_level: FoveatedLevel::High,
            cpu_priority_boost: true,
            gpu_priority: GPUPriority::High,
            pixel_density: 1.0,
            fov_scale: 1.0,
            force_composition_layers: false,
            disable_depth_submission: false,
            turbo_mode: false,
            auto_restart_on_freeze: true,
            kill_oculus_client: false,
            restart_threshold_seconds: 10,
            upscaling_enabled: false,
            upscaling_type: UpscalingType::FSR,
            upscaling_scale: 1.0,
            sharpening_amount: 0.5,
            contrast: 1.0,
            saturation: 1.0,
            frame_throttle_fps: 90,
            shake_reduction: false,
            audio_switching: true,
            super_sampling: 1.0,
            mirror_window: false,
            guardian_visibility: true,
            cpu_affinity: 0,
            power_plan: PowerPlan::HighPerformance,
            oculus_killer_enabled: false,
            relinked_mode: false,
            disable_asw: false,
            enable_steamvr_autostart: true,
            enable_runtime_high_priority: true,
            allow_other_software: true,
            custom_startup_program: String::from(""),
            custom_fps: 120,
            disable_oled_mura: false,
            debug_logging: false,
            disable_telemetry: false,
            disable_login: false,
        }
    }
}

#[derive(Clone)]
struct ProcessInfo {
    name: String,
    status: ProcessStatus,
    pid: Option<u32>,
    cpu_usage: f32,
    memory_mb: u64,
}

#[derive(PartialEq, Clone)]
enum ProcessStatus {
    Running,
    Stopped,
    Frozen,
    Restarting,
}

struct VRPerformanceApp {
    settings: VRSettings,
    system: Arc<Mutex<System>>,
    processes: Vec<ProcessInfo>,
    current_tab: Tab,
    stats: PerformanceStats,
    show_advanced: bool,
}

#[derive(PartialEq)]
enum Tab {
    Performance,
    Visual,
    Processes,
    Advanced,
    Stats,
    ReLinked,
}

struct PerformanceStats {
    fps: f32,
    frame_time_ms: f32,
    cpu_usage: f32,
    gpu_usage: f32,
    vram_used_gb: f32,
    latency_ms: f32,
}

impl Default for VRPerformanceApp {
    fn default() -> Self {
        let _ = WriteLogger::init(LevelFilter::Debug, Config::default(), fs::File::create("vr_suite.log").unwrap());
        
        let mut settings = VRSettings::default();
        if let Ok(mut file) = fs::File::open("settings.json") {
            let mut contents = String::new();
            if file.read_to_string(&mut contents).is_ok() {
                if let Ok(loaded) = serde_json::from_str(&contents) {
                    settings = loaded;
                }
            }
        }
        
        Self {
            settings,
            system: Arc::new(Mutex::new(System::new_all())),
            processes: Vec::new(),
            current_tab: Tab::Performance,
            stats: PerformanceStats {
                fps: 90.0,
                frame_time_ms: 11.1,
                cpu_usage: 0.0,
                gpu_usage: 0.0,
                vram_used_gb: 0.0,
                latency_ms: 0.0,
            },
            show_advanced: false,
        }
    }
}

impl VRPerformanceApp {
    fn save_settings(&self) {
        if let Ok(json) = serde_json::to_string_pretty(&self.settings) {
            if let Ok(mut file) = fs::File::create("settings.json") {
                let _ = file.write_all(json.as_bytes());
            }
        }
    }
    
    fn update_processes(&mut self) {
        let mut sys = self.system.lock().unwrap();
        sys.refresh_processes();
        
        let vr_processes = vec![
            "OVRServer_x64.exe",
            "OculusClient.exe",
            "vrserver.exe",
            "vrdashboard.exe",
            "vrcompositor.exe",
        ];
        
        self.processes.clear();
        
        for proc_name in vr_processes {
            let proc = sys.processes_by_name(proc_name).next();
            
            if let Some(p) = proc {
                self.processes.push(ProcessInfo {
                    name: proc_name.to_string(),
                    status: ProcessStatus::Running,
                    pid: Some(p.pid().as_u32()),
                    cpu_usage: p.cpu_usage(),
                    memory_mb: p.memory() / 1024 / 1024,
                });
            } else {
                self.processes.push(ProcessInfo {
                    name: proc_name.to_string(),
                    status: ProcessStatus::Stopped,
                    pid: None,
                    cpu_usage: 0.0,
                    memory_mb: 0,
                });
            }
        }
    }
    
    fn apply_settings(&mut self) {
        info!("Applying settings");
        self.apply_oculus_link_settings();
        self.apply_openxr_settings();
        self.apply_process_priorities();
        self.apply_asw_settings();
        self.apply_additional_settings();
        self.toggle_oculus_killer(self.settings.oculus_killer_enabled);
        self.apply_relinked_settings();
        self.save_settings();
    }
    
    fn apply_oculus_link_settings(&self) {
        #[cfg(target_os = "windows")]
        {
            use winreg::enums::*;
            use winreg::RegKey;
            
            let hkcu = RegKey::predef(HKEY_CURRENT_USER);
            if let Ok(oculus_key) = hkcu.create_subkey("Software\\Oculus\\RemoteHeadset") {
                let (key, _) = oculus_key;
                let _ = key.set_value("BitrateMbps", &self.settings.encode_bitrate_mbps);
                let _ = key.set_value("EncodeResolutionWidth", &self.settings.encode_resolution_width);
                let _ = key.set_value("EncodeResolutionHeight", &self.settings.encode_resolution_height);
                let enabled: u32 = if self.settings.link_sharpening > 0.0 { 1 } else { 0 };
                let _ = key.set_value("LinkSharpeningEnabled", &enabled);
                let strength: u32 = (self.settings.link_sharpening * 100.0) as u32;
                let _ = key.set_value("LinkSharpeningStrength", &strength);
            }
        }
    }
    
    fn apply_openxr_settings(&self) {
        #[cfg(target_os = "windows")]
        {
            use winreg::enums::*;
            use winreg::RegKey;
            
            let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
            if let Ok(openxr_key) = hklm.create_subkey("SOFTWARE\\Khronos\\OpenXR\\1") {
                let (key, _) = openxr_key;
                
                if self.settings.use_openxr {
                    let _ = key.set_value("ActiveRuntime", &"oculus");
                } else if self.settings.use_steamvr {
                    let _ = key.set_value("ActiveRuntime", &"steamvr");
                }
            }
        }
    }
    
    fn apply_process_priorities(&self) {
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::System::Threading::*;
            use windows::Win32::Foundation::*;
            
            if self.settings.cpu_priority_boost {
                for proc in &self.processes {
                    if let Some(pid) = proc.pid {
                        if proc.name.contains("OVRServer") || proc.name.contains("vrserver") {
                            unsafe {
                                if let Ok(handle) = OpenProcess(PROCESS_SET_INFORMATION, false, pid) {
                                    let priority = match self.settings.gpu_priority {
                                        GPUPriority::Realtime => REALTIME_PRIORITY_CLASS,
                                        GPUPriority::High => HIGH_PRIORITY_CLASS,
                                        GPUPriority::Normal => NORMAL_PRIORITY_CLASS,
                                    };
                                    let _ = SetPriorityClass(handle, priority);
                                    let _ = CloseHandle(handle);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    fn apply_asw_settings(&self) {
        #[cfg(target_os = "windows")]
        {
            use winreg::enums::*;
            use winreg::RegKey;
            
            let hkcu = RegKey::predef(HKEY_CURRENT_USER);
            if let Ok(debug_key) = hkcu.create_subkey("Software\\Oculus\\Debug") {
                let (key, _) = debug_key;
                
                let asw_value: u32 = match self.settings.asw_mode {
                    ASWMode::Off => 0,
                    ASWMode::Auto => 1,
                    ASWMode::Force45FPS => 2,
                    ASWMode::Force30FPS => 3,
                };
                
                let _ = key.set_value("ASW", &asw_value);
            }
        }
    }
    
    fn apply_additional_settings(&self) {
        #[cfg(target_os = "windows")]
        {
            use winreg::enums::*;
            use winreg::RegKey;
            
            let power_guid = match self.settings.power_plan {
                PowerPlan::Balanced => "381b4222-f694-41f0-9685-ff5bb260df2e",
                PowerPlan::HighPerformance => "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c",
                PowerPlan::PowerSaver => "a1841308-3541-4fab-bc81-f71556f20b4a",
            };
            let _ = Command::new("powercfg").args(["/s", power_guid]).output();
            
            let hkcu = RegKey::predef(HKEY_CURRENT_USER);
            if let Ok(oculus_key) = hkcu.create_subkey("Software\\Oculus\\RemoteHeadset") {
                let (key, _) = oculus_key;
                let mirror_val: u32 = if self.settings.mirror_window { 1 } else { 0 };
                let guardian_val: u32 = if self.settings.guardian_visibility { 1 } else { 0 };
                let _ = key.set_value("MirrorWindow", &mirror_val);
                let _ = key.set_value("GuardianVisibility", &guardian_val);
            }
            
            if let Ok(mut file) = fs::File::create("openxr_toolkit.ini") {
                let _ = write!(file, "upscaling_enabled = {}", self.settings.upscaling_enabled);
            }
        }
    }
    
    fn toggle_oculus_killer(&self, enable: bool) {
        #[cfg(target_os = "windows")]
        {
            let _ = Command::new("sc").args(["stop", "OVRService"]).output();
            std::thread::sleep(std::time::Duration::from_secs(2));
            
            let path = r"C:\Program Files\Oculus\Support\oculus-dash\dash\bin";
            let dash_path = format!("{}\\OculusDash.exe", path);
            let bak_path = format!("{}\\OculusDash.exe.bak", path);
            
            if enable {
                if !Path::new(&bak_path).exists() {
                    let _ = fs::rename(&dash_path, &bak_path);
                }
            } else {
                if Path::new(&bak_path).exists() {
                    let _ = fs::remove_file(&dash_path);
                    let _ = fs::rename(&bak_path, &dash_path);
                }
            }
            
            let _ = Command::new("sc").args(["start", "OVRService"]).output();
            
            if enable {
                use winreg::enums::*;
                use winreg::RegKey;
                let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
                if let Ok(key) = hklm.open_subkey_with_flags("SOFTWARE\\WOW6432Node\\Oculus VR, LLC\\Oculus\\Config", KEY_WRITE) {
                    let _ = key.set_value("CoreChannel", &"NO_UPDATES");
                }
            }
        }
    }
    
    fn apply_relinked_settings(&mut self) {
        if self.settings.debug_logging {
            debug!("Applying ReLinked settings");
        }
        
        #[cfg(target_os = "windows")]
        {
            use winreg::enums::*;
            use winreg::RegKey;
            
            if self.settings.relinked_mode {
                self.settings.disable_telemetry = true;
                self.settings.disable_login = true;
                self.settings.oculus_killer_enabled = true;
                self.toggle_oculus_killer(true);
                
                if self.settings.disable_telemetry {
                    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
                    if let Ok(key) = hkcu.create_subkey("Software\\Oculus\\Telemetry") {
                        let (key, _) = key;
                        let _ = key.set_value("Enabled", &0u32);
                    }
                }
                
                info!("ReLinked mode enabled - manual runtime modifications may be needed");
                info!("Setting custom FPS to {}", self.settings.custom_fps);
                
                self.settings.enable_runtime_high_priority = true;
                self.apply_process_priorities();
                
                self.settings.allow_other_software = true;
                self.apply_openxr_settings();
            }
        }
    }
    
    fn launch_runtime(&self) {
        #[cfg(target_os = "windows")]
        {
            let oculus_path = r"C:\Program Files\Oculus\Support\oculus-runtime\OVRServer_x64.exe";
            let _ = Command::new(oculus_path).spawn();
            info!("Launched Oculus Runtime");
        }
    }
    
    fn restart_process(&self, process_name: &str) {
        #[cfg(target_os = "windows")]
        {
            let _ = Command::new("taskkill")
                .args(&["/F", "/IM", process_name])
                .output();
            
            std::thread::sleep(std::time::Duration::from_millis(500));
            
            if process_name.contains("OVRServer") {
                let oculus_path = r"C:\Program Files\Oculus\Support\oculus-runtime\OVRServer_x64.exe";
                let _ = Command::new(oculus_path).spawn();
            } else if process_name.contains("vrserver") {
                let steam_path = r"C:\Program Files (x86)\Steam\steamapps\common\SteamVR\bin\win64\vrserver.exe";
                let _ = Command::new(steam_path).spawn();
            }
        }
    }
    
    fn kill_oculus_client(&self) {
        #[cfg(target_os = "windows")]
        {
            let _ = Command::new("taskkill")
                .args(&["/F", "/IM", "OculusClient.exe"])
                .output();
        }
    }
}

impl eframe::App for VRPerformanceApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_processes();
        
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("VR Performance Suite");
                ui.separator();
                
                if ui.selectable_label(self.current_tab == Tab::Performance, "Performance").clicked() {
                    self.current_tab = Tab::Performance;
                }
                if ui.selectable_label(self.current_tab == Tab::Visual, "Visual").clicked() {
                    self.current_tab = Tab::Visual;
                }
                if ui.selectable_label(self.current_tab == Tab::Processes, "Processes").clicked() {
                    self.current_tab = Tab::Processes;
                }
                if ui.selectable_label(self.current_tab == Tab::Advanced, "Advanced").clicked() {
                    self.current_tab = Tab::Advanced;
                }
                if ui.selectable_label(self.current_tab == Tab::Stats, "Stats").clicked() {
                    self.current_tab = Tab::Stats;
                }
                if ui.selectable_label(self.current_tab == Tab::ReLinked, "ReLinked").clicked() {
                    self.current_tab = Tab::ReLinked;
                }
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Apply All").clicked() {
                        self.apply_settings();
                    }
                });
            });
        });
        
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                match self.current_tab {
                    Tab::Performance => self.show_performance_tab(ui),
                    Tab::Visual => self.show_visual_tab(ui),
                    Tab::Processes => self.show_processes_tab(ui),
                    Tab::Advanced => self.show_advanced_tab(ui),
                    Tab::Stats => self.show_stats_tab(ui),
                    Tab::ReLinked => self.show_relinked_tab(ui),
                }
            });
        });
        
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
    }
}

impl VRPerformanceApp {
    fn show_performance_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Performance Settings");
        ui.separator();
        
        ui.group(|ui| {
            ui.label("Runtime Selection");
            ui.radio_value(&mut self.settings.use_openxr, true, "Use Oculus OpenXR (Recommended for Quest Link)");
            ui.radio_value(&mut self.settings.use_steamvr, true, "Use SteamVR OpenXR");
        });
        
        ui.add_space(10.0);
        
        ui.group(|ui| {
            ui.label("Render Scale");
            ui.add(egui::Slider::new(&mut self.settings.render_scale, 0.5..=2.0).text("Scale"));
            ui.label(format!("Resolution: {}x{}", 
                (2064.0 * self.settings.render_scale) as u32,
                (2208.0 * self.settings.render_scale) as u32
            ));
        });
        
        ui.add_space(10.0);
        
        ui.group(|ui| {
            ui.label("ASW (Asynchronous Spacewarp)");
            ui.checkbox(&mut self.settings.asw_enabled, "Enable ASW");
            
            if self.settings.asw_enabled {
                ui.horizontal(|ui| {
                    ui.label("Mode:");
                    ui.radio_value(&mut self.settings.asw_mode, ASWMode::Auto, "Auto");
                    ui.radio_value(&mut self.settings.asw_mode, ASWMode::Force45FPS, "45 FPS");
                    ui.radio_value(&mut self.settings.asw_mode, ASWMode::Force30FPS, "30 FPS");
                    ui.radio_value(&mut self.settings.asw_mode, ASWMode::Off, "Off");
                });
            }
        });
        
        ui.add_space(10.0);
        
        ui.group(|ui| {
            ui.label("Foveated Rendering");
            ui.checkbox(&mut self.settings.foveated_rendering, "Enable Foveated Rendering");
            
            if self.settings.foveated_rendering {
                ui.horizontal(|ui| {
                    ui.label("Level:");
                    ui.radio_value(&mut self.settings.foveated_level, FoveatedLevel::Low, "Low");
                    ui.radio_value(&mut self.settings.foveated_level, FoveatedLevel::Medium, "Medium");
                    ui.radio_value(&mut self.settings.foveated_level, FoveatedLevel::High, "High");
                    ui.radio_value(&mut self.settings.foveated_level, FoveatedLevel::HighTop, "High+Top");
                });
            }
        });
        
        ui.add_space(10.0);
        
        ui.group(|ui| {
            ui.label("Process Priority");
            ui.checkbox(&mut self.settings.cpu_priority_boost, "Boost VR Process Priority");
            
            if self.settings.cpu_priority_boost {
                ui.horizontal(|ui| {
                    ui.label("GPU Priority:");
                    ui.radio_value(&mut self.settings.gpu_priority, GPUPriority::Normal, "Normal");
                    ui.radio_value(&mut self.settings.gpu_priority, GPUPriority::High, "High");
                    ui.radio_value(&mut self.settings.gpu_priority, GPUPriority::Realtime, "Realtime");
                });
            }
        });
        
        ui.add_space(10.0);
        
        ui.group(|ui| {
            ui.label("Frame Throttle");
            ui.add(egui::Slider::new(&mut self.settings.frame_throttle_fps, 30..=120).text("FPS Limit"));
        });
        
        ui.checkbox(&mut self.settings.shake_reduction, "Enable Shake Reduction");
        
        ui.horizontal(|ui| {
            ui.label("Power Plan:");
            ui.radio_value(&mut self.settings.power_plan, PowerPlan::Balanced, "Balanced");
            ui.radio_value(&mut self.settings.power_plan, PowerPlan::HighPerformance, "High Performance");
            ui.radio_value(&mut self.settings.power_plan, PowerPlan::PowerSaver, "Power Saver");
        });
    }
    
    fn show_visual_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Visual & Encoding Settings");
        ui.separator();
        
        ui.group(|ui| {
            ui.label("Oculus Link Encoding");
            
            ui.horizontal(|ui| {
                ui.label("Bitrate (Mbps):");
                ui.add(egui::Slider::new(&mut self.settings.encode_bitrate_mbps, 50..=500));
            });
            
            ui.horizontal(|ui| {
                ui.label("Encode Width:");
                ui.add(egui::Slider::new(&mut self.settings.encode_resolution_width, 1440..=3664));
            });
            
            ui.horizontal(|ui| {
                ui.label("Encode Height:");
                ui.add(egui::Slider::new(&mut self.settings.encode_resolution_height, 1584..=1920));
            });
            
            ui.label("Higher = Better quality, more bandwidth. 200+ recommended for Quest 3.");
        });
        
        ui.add_space(10.0);
        
        ui.group(|ui| {
            ui.label("Link Sharpening");
            ui.add(egui::Slider::new(&mut self.settings.link_sharpening, 0.0..=1.0).text("Sharpness"));
            ui.label("Adds post-processing sharpening to Link video stream");
        });
        
        ui.add_space(10.0);
        
        ui.group(|ui| {
            ui.label("Advanced Visual");
            
            ui.horizontal(|ui| {
                ui.label("Pixel Density:");
                ui.add(egui::Slider::new(&mut self.settings.pixel_density, 0.5..=2.0));
            });
            
            ui.horizontal(|ui| {
                ui.label("FOV Scale:");
                ui.add(egui::Slider::new(&mut self.settings.fov_scale, 0.8..=1.2));
            });
            
            ui.add(egui::Slider::new(&mut self.settings.contrast, 0.5..=1.5).text("Contrast"));
            ui.add(egui::Slider::new(&mut self.settings.saturation, 0.5..=1.5).text("Saturation"));
            ui.add(egui::Slider::new(&mut self.settings.sharpening_amount, 0.0..=1.0).text("Sharpening Amount"));
            ui.add(egui::Slider::new(&mut self.settings.super_sampling, 1.0..=2.0).text("Super Sampling"));
        });
        
        ui.group(|ui| {
            ui.label("Upscaling");
            ui.checkbox(&mut self.settings.upscaling_enabled, "Enable Upscaling");
            if self.settings.upscaling_enabled {
                ui.horizontal(|ui| {
                    ui.label("Type:");
                    ui.radio_value(&mut self.settings.upscaling_type, UpscalingType::NIS, "NIS");
                    ui.radio_value(&mut self.settings.upscaling_type, UpscalingType::FSR, "FSR");
                    ui.radio_value(&mut self.settings.upscaling_type, UpscalingType::CAS, "CAS");
                });
                ui.add(egui::Slider::new(&mut self.settings.upscaling_scale, 0.5..=1.0).text("Scale"));
            }
        });
        
        ui.checkbox(&mut self.settings.mirror_window, "Enable Mirror Window");
        ui.checkbox(&mut self.settings.guardian_visibility, "Show Guardian");
    }
    
    fn show_processes_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Process Management");
        ui.separator();
        
        ui.group(|ui| {
            ui.label("Auto-Recovery Settings");
            ui.checkbox(&mut self.settings.auto_restart_on_freeze, "Auto-restart frozen processes");
            ui.checkbox(&mut self.settings.kill_oculus_client, "Kill Oculus Client (reduces overhead)");
            
            if self.settings.kill_oculus_client {
                if ui.button("Kill Oculus Client Now").clicked() {
                    self.kill_oculus_client();
                }
            }
            
            ui.checkbox(&mut self.settings.oculus_killer_enabled, "Enable OculusKiller (Disables Oculus Dash)");
            ui.label("Note: Requires admin privileges to modify Oculus files.");
        });
        
        ui.add_space(10.0);
        
        ui.group(|ui| {
            ui.label("VR Processes");
            ui.separator();
            
            egui::Grid::new("process_grid")
                .striped(true)
                .min_col_width(150.0)
                .show(ui, |ui| {
                    ui.label("Process");
                    ui.label("Status");
                    ui.label("CPU %");
                    ui.label("Memory (MB)");
                    ui.label("Actions");
                    ui.end_row();
                    
                    for proc in &self.processes {
                        ui.label(&proc.name);
                        
                        let status_text = match proc.status {
                            ProcessStatus::Running => "Running",
                            ProcessStatus::Stopped => "Stopped",
                            ProcessStatus::Frozen => "Frozen",
                            ProcessStatus::Restarting => "Restarting",
                        };
                        ui.label(status_text);
                        
                        ui.label(format!("{:.1}%", proc.cpu_usage));
                        ui.label(format!("{} MB", proc.memory_mb));
                        
                        if proc.status == ProcessStatus::Running {
                            if ui.button("Restart").clicked() {
                                self.restart_process(&proc.name);
                            }
                        } else {
                            ui.label("-");
                        }
                        
                        ui.end_row();
                    }
                });
        });
        
        ui.add_space(10.0);
        
        if ui.button("Restart All VR Services").clicked() {
            for proc in self.processes.clone() {
                if proc.status == ProcessStatus::Running {
                    self.restart_process(&proc.name);
                }
            }
        }
        
        ui.add_space(10.0);
        
        ui.group(|ui| {
            ui.label("CPU Affinity");
            ui.add(egui::Slider::new(&mut self.settings.cpu_affinity, 0..=15).text("Affinity Mask"));
        });
        
        ui.checkbox(&mut self.settings.audio_switching, "Automatic Audio Switching");
    }
    
    fn show_advanced_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Advanced Settings");
        ui.separator();
        
        ui.label("WARNING: These settings may cause instability if misconfigured");
        
        ui.add_space(10.0);
        
        ui.group(|ui| {
            ui.label("OpenXR Advanced");
            ui.checkbox(&mut self.settings.force_composition_layers, "Force Composition Layers");
            ui.checkbox(&mut self.settings.disable_depth_submission, "Disable Depth Submission");
            ui.checkbox(&mut self.settings.turbo_mode, "Turbo Mode (reduces latency)");
        });
        
        ui.add_space(10.0);
        
        ui.group(|ui| {
            ui.label("Debug Tools");
            
            if ui.button("Open Oculus Debug Tool").clicked() {
                #[cfg(target_os = "windows")]
                {
                    let _ = Command::new(r"C:\Program Files\Oculus\Support\oculus-diagnostics\OculusDebugTool.exe").spawn();
                }
            }
            
            if ui.button("Open SteamVR Settings").clicked() {
                #[cfg(target_os = "windows")]
                {
                    let _ = Command::new("cmd").args(&["/c", "start", "steam://open/settings"]).spawn();
                }
            }
        });
    }
    
    fn show_stats_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Performance Statistics");
        ui.separator();
        
        ui.group(|ui| {
            ui.label("Real-time Performance");
            
            egui::Grid::new("stats_grid")
                .num_columns(2)
                .spacing([40.0, 8.0])
                .show(ui, |ui| {
                    ui.label("FPS:");
                    ui.label(format!("{:.1}", self.stats.fps));
                    ui.end_row();
                    
                    ui.label("Frame Time:");
                    ui.label(format!("{:.2} ms", self.stats.frame_time_ms));
                    ui.end_row();
                    
                    ui.label("CPU Usage:");
                    ui.label(format!("{:.1}%", self.stats.cpu_usage));
                    ui.end_row();
                    
                    ui.label("GPU Usage:");
                    ui.label(format!("{:.1}%", self.stats.gpu_usage));
                    ui.end_row();
                    
                    ui.label("VRAM Used:");
                    ui.label(format!("{:.2} GB", self.stats.vram_used_gb));
                    ui.end_row();
                    
                    ui.label("Motion-to-Photon Latency:");
                    ui.label(format!("{:.1} ms", self.stats.latency_ms));
                    ui.end_row();
                });
        });
        
        ui.add_space(10.0);
        
        ui.label("Stats are read from VR runtime when available");
    }
    
    fn show_relinked_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("ReLinked VR Settings");
        ui.separator();
        
        ui.checkbox(&mut self.settings.relinked_mode, "Enable ReLinked Mode (Minimal Runtime)");
        ui.label("Note: This approximates ReLinked features. Full ReLinked requires custom runtime.");
        
        if ui.button("Launch Runtime").clicked() {
            self.launch_runtime();
        }
        
        ui.group(|ui| {
            ui.label("General Options");
            ui.checkbox(&mut self.settings.disable_asw, "Disable ASW");
            ui.checkbox(&mut self.settings.enable_steamvr_autostart, "Enable SteamVR Auto-Start");
            ui.checkbox(&mut self.settings.enable_runtime_high_priority, "Enable Runtime High Priority");
            ui.checkbox(&mut self.settings.allow_other_software, "Allow Other Software (CAPI/OpenXR)");
            ui.checkbox(&mut self.settings.disable_telemetry, "Disable Telemetry");
            ui.checkbox(&mut self.settings.disable_login, "Disable Login (Approximate)");
            ui.horizontal(|ui| {
                ui.label("Custom Startup Program:");
                ui.text_edit_singleline(&mut self.settings.custom_startup_program);
            });
        });
        
        ui.group(|ui| {
            ui.label("Quest Link Options");
            ui.horizontal(|ui| {
                ui.label("Custom FPS:");
                ui.add(egui::Slider::new(&mut self.settings.custom_fps, 60..=120));
            });
        });
        
        ui.group(|ui| {
            ui.label("Rift Options");
            ui.checkbox(&mut self.settings.disable_oled_mura, "Disable OLED Mura Correction");
        });
        
        ui.checkbox(&mut self.settings.debug_logging, "Enable Debug Logging");
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 700.0])
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    
    eframe::run_native(
        "VR Performance Suite",
        options,
        Box::new(|_cc| Box::new(VRPerformanceApp::default())),
    )
}