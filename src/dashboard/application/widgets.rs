use eframe::egui::Ui;

pub enum Modal {
    None,
    Message {
        title: String,
        message: String,
    },
    YesNo {
        title: String,
        question: String,
        on_yes: Box<dyn FnMut()>,
        on_no: Box<dyn FnMut()>,
    },
    Custom {
        content: Box<dyn FnMut(&mut Ui)>,
    },
}
