use super::hotkey::HotkeyManager;
use super::state::{AppState, StateManager};
use crate::ffi::OnlineStream;
use crate::injector::TextInjector;
use crate::recognizer::{ProcessOutcome, RecognizerEngine};
use crate::vad::VadConfig;
use crate::wayland;
use eframe::egui;
use std::sync::{Arc, Mutex};

/// Cinnabar GUI 悬浮窗
pub struct CinnabarWindow {
    state_manager: Arc<Mutex<StateManager>>,
    hotkey_manager: Option<Arc<Mutex<HotkeyManager>>>,
    recognizer: Option<RecognizerEngine>,
    stream: Option<OnlineStream>,
    injector: Option<TextInjector>,
}

impl CinnabarWindow {
    /// 创建新的悬浮窗实例
    pub fn new(_cc: &eframe::CreationContext<'_>, model_dir: &std::path::Path) -> Self {
        // GUI 也走 ten-vad：构造默认 VadConfig，路径相对 model_dir。
        let mut vad_cfg = VadConfig::default();
        vad_cfg.model_path = model_dir
            .join("ten-vad.onnx")
            .to_string_lossy()
            .into_owned();

        let mut recognizer =
            RecognizerEngine::new(model_dir, None, None, vad_cfg).ok();

        if let Some(ref mut r) = recognizer {
            r.start();
        }

        let stream = recognizer.as_ref().map(|r| r.create_stream());
        let injector = TextInjector::new().ok();

        Self {
            state_manager: Arc::new(Mutex::new(StateManager::new())),
            hotkey_manager: None,
            recognizer,
            stream,
            injector,
        }
    }

    /// 获取状态管理器的引用
    pub fn state_manager(&self) -> Arc<Mutex<StateManager>> {
        Arc::clone(&self.state_manager)
    }

    /// 设置热键管理器
    pub fn set_hotkey_manager(&mut self, manager: Arc<Mutex<HotkeyManager>>) {
        self.hotkey_manager = Some(manager);
    }
}

impl eframe::App for CinnabarWindow {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 处理热键事件
        if let Some(ref hotkey_manager) = self.hotkey_manager {
            hotkey_manager.lock().unwrap().handle_events();
        }

        // 窗口定位
        if let Ok(win_info) = wayland::get_active_window() {
            let pos = egui::pos2(
                (win_info.x + win_info.width as i32 / 2 - 125) as f32,
                (win_info.y - 160) as f32,
            );
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos));
        }

        // 处理语音识别
        let current_state = self.state_manager.lock().unwrap().get_state();

        if current_state == AppState::Listening {
            if let (Some(ref mut recognizer), Some(ref mut stream)) =
                (&mut self.recognizer, &mut self.stream)
            {
                // ten-vad 决定何时结束 segment；不再用标点符号触发。
                match recognizer.process(stream) {
                    ProcessOutcome::Idle => {}
                    ProcessOutcome::Partial(text) => {
                        let sm = self.state_manager.lock().unwrap();
                        sm.set_text(text);
                        sm.set_state(AppState::Recognizing);
                    }
                    ProcessOutcome::SegmentEnded { text, segment_id: _ } => {
                        let sm = self.state_manager.lock().unwrap();
                        sm.set_text(text.clone());
                        sm.set_state(AppState::Injecting);
                        drop(sm);

                        // 注入文本
                        if let Some(ref mut injector) = self.injector {
                            let _ = injector.paste_text(&text);
                        }

                        // 绕开 sherpa-onnx 1.12.9 reset 崩溃：销毁并重建 ASR 流。
                        *stream = recognizer.create_stream();
                        recognizer.reset_vad();

                        // 返回待机状态
                        let sm = self.state_manager.lock().unwrap();
                        sm.set_state(AppState::Idle);
                        sm.clear_text();
                    }
                }
            }
        }

        let state_manager = self.state_manager.lock().unwrap();

        // 设置窗口样式
        egui::CentralPanel::default().show(ctx, |ui| {
            // 标题栏
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("{} Cinnabar", state_manager.get_state_icon()))
                        .size(18.0)
                        .strong(),
                );
            });

            ui.separator();

            // 状态显示
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new(format!("状态: {}", state_manager.get_state_description()))
                        .size(14.0),
                );

                ui.add_space(8.0);

                // 根据状态显示不同内容
                match state_manager.get_state() {
                    AppState::Idle => {
                        ui.label(
                            egui::RichText::new("按 F3 开始")
                                .size(12.0)
                                .color(egui::Color32::GRAY),
                        );
                    }
                    AppState::Listening => {
                        ui.label(
                            egui::RichText::new("按 F3 停止")
                                .size(12.0)
                                .color(egui::Color32::RED),
                        );
                    }
                    AppState::Recognizing => {
                        let text = state_manager.get_text();
                        if !text.is_empty() {
                            ui.label(
                                egui::RichText::new(&text)
                                    .size(12.0)
                                    .color(egui::Color32::GREEN),
                            );
                        } else {
                            ui.label(
                                egui::RichText::new("识别中...")
                                    .size(12.0)
                                    .color(egui::Color32::GREEN),
                            );
                        }
                    }
                    AppState::Injecting => {
                        ui.label(
                            egui::RichText::new("已注入文本")
                                .size(12.0)
                                .color(egui::Color32::LIGHT_GREEN),
                        );
                        let text = state_manager.get_text();
                        if !text.is_empty() {
                            ui.label(
                                egui::RichText::new(&text)
                                    .size(11.0)
                                    .color(egui::Color32::GRAY),
                            );
                        }
                    }
                }
            });
        });

        // 请求持续重绘以保持 UI 响应
        ctx.request_repaint();
    }
}
