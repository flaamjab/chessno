use jni::{self, objects::JObject, JavaVM};
use ndk_context::{self, AndroidContext};

use crate::logging::warn;

const ALERT_DIALOG_BUILDER_CLASS: &str = "android/app/AlertDialog$Builder";
const CONTEXT_CLASS: &str = "android/content/Context";

pub struct AlertDialog {
    ctx: AndroidContext,
    vm: JavaVM,
}

impl AlertDialog {
    pub fn new() -> Self {
        let ctx = ndk_context::android_context();
        let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }.unwrap();

        Self { ctx, vm }
    }

    pub fn show(&self, message: &str) {
        let env = self.vm.attach_current_thread().unwrap();
        let alert_dialog_builder_class = env.find_class(ALERT_DIALOG_BUILDER_CLASS).unwrap();
        unsafe {
            let context_object = JObject::from_raw(self.ctx.context().cast());
            let alert_dialog_builder = env
                .new_object(
                    alert_dialog_builder_class,
                    format!("(L{};)V", CONTEXT_CLASS),
                    &[context_object.into()],
                )
                .unwrap();
        }
    }
}
