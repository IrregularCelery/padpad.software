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
        on_yes: Option<Box<dyn FnMut()>>,
        on_no: Option<Box<dyn FnMut()>>,
    },
    Custom {
        content: Box<dyn FnMut(&mut Ui)>,
    },
}
