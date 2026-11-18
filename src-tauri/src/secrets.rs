use crate::domain::Provider;
use std::{collections::HashMap, sync::Mutex};

#[derive(Default)]
pub struct Secrets {
    session: Mutex<HashMap<String, String>>,
}
impl Secrets {
    pub fn set(&self, provider: &Provider, value: String) -> anyhow::Result<bool> {
        anyhow::ensure!(
            *provider != Provider::Local,
            "Local models do not require an API key."
        );
        anyhow::ensure!(
            !value.trim().is_empty() && value.len() < 4096,
            "Invalid API key."
        );
        self.session
            .lock()
            .unwrap()
            .insert(provider.key().into(), value.clone());
        let stored = keyring::Entry::new("de.meetingpal.desktop", provider.key())
            .and_then(|entry| entry.set_password(&value))
            .is_ok();
        Ok(stored)
    }
    pub fn get(&self, provider: &Provider) -> anyhow::Result<String> {
        if let Some(value) = self.session.lock().unwrap().get(provider.key()) {
            return Ok(value.clone());
        }
        keyring::Entry::new("de.meetingpal.desktop", provider.key())
            .and_then(|entry| entry.get_password())
            .map_err(|_| {
                anyhow::anyhow!(
                    "No API key available for {}. Add it in Settings.",
                    provider.key()
                )
            })
    }
    pub fn delete(&self, provider: &Provider) -> anyhow::Result<()> {
        self.session.lock().unwrap().remove(provider.key());
        if let Ok(entry) = keyring::Entry::new("de.meetingpal.desktop", provider.key()) {
            match entry.delete_credential() {
                Ok(_) | Err(keyring::Error::NoEntry) => {}
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    }
    pub fn status(&self) -> HashMap<String, bool> {
        [Provider::Openai, Provider::Anthropic, Provider::Gemini]
            .iter()
            .map(|p| (p.key().into(), self.get(p).is_ok()))
            .collect()
    }
}
