//! Safe JNI calls into the small Android document-picker host.
#![forbid(unsafe_code)]

use jni::objects::{JObject, JString, JValue};

fn call<R>(f: impl FnOnce(&mut jni::JNIEnv<'_>, &JObject<'_>) -> jni::errors::Result<R>) -> Result<R, String> {
    robius_android_env::with_activity(|env, activity| {
        // The native event thread stays attached to the VM. Release each call's temporary
        // Java references so polling the document queues cannot fill the local-reference table.
        let result = env.with_local_frame(16, |env| f(env, activity));
        if result.is_err() {
            let _ = env.exception_clear();
        }
        result
    })
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

pub fn pick_open() -> Result<(), String> {
    call(|env, activity| env.call_method(activity, "pickDrawing", "()V", &[]).map(|_| ()))
}

pub fn input(text: &str) -> Result<(), String> {
    call(|env, activity| {
        let text = JObject::from(env.new_string(text)?);
        env.call_method(activity, "enterCommand", "(Ljava/lang/String;)V", &[JValue::Object(&text)])?;
        Ok(())
    })
}

pub fn export(path: &str, name: &str) -> Result<(), String> {
    call(|env, activity| {
        let path = JObject::from(env.new_string(path)?);
        let name = JObject::from(env.new_string(name)?);
        env.call_method(activity, "exportDrawing", "(Ljava/lang/String;Ljava/lang/String;)V", &[JValue::Object(&path), JValue::Object(&name)])?;
        Ok(())
    })
}

pub fn take(method: &str) -> Result<Option<String>, String> {
    call(|env, activity| {
        let value = env.call_method(activity, method, "()Ljava/lang/String;", &[])?.l()?;
        if value.is_null() {
            return Ok(None);
        }
        let text = JString::from(value);
        Ok(Some(env.get_string(&text)?.into()))
    })
}
