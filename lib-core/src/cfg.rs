use std::sync::{Arc, RwLock};

use state::TypeMap;

/// A configuration map.
pub struct ConfigMap(Arc<RwLock<TypeMap![Sync + Send]>>);
impl ConfigMap {
    /// Fetches a configuration holder for a type that accepts stored config.
    /// # Panics
    /// Panics if the read-write lock is poisoned.
    #[must_use]
    pub fn fetch<AC: AcceptsConfig>(&self) -> ConfigHolder<AC> {
        let ch = self.0.read().unwrap().try_get::<ConfigHolder<AC>>().cloned();
        ch.map_or_else(move || {
            let cfg = ConfigHolder(Arc::new(AC::Cfg::default()));
            self.0.write().unwrap().set::<ConfigHolder<AC>>(ConfigHolder::clone(&cfg));
            cfg
        }, |cfg| cfg)
    }

    /// Sets a config for a type that accepts it. It should be used before any configurable operation.
    /// # Panics
    /// Panics if the read-write lock is poisoned.
    pub fn set<AC: AcceptsConfig>(&self, cfg: AC::Cfg) {
        self.0.write().unwrap().set::<ConfigHolder<AC>>(ConfigHolder(Arc::new(cfg)));
    }
}
impl Clone for ConfigMap {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl Default for ConfigMap {
    fn default() -> Self {
        Self(Arc::new(RwLock::new(<TypeMap![Sync + Send]>::new())))
    }
}

/// A type that accepts a stored config. It usually is an empty enum.
pub trait AcceptsConfig: 'static {
    /// A configuration type.
    type Cfg: 'static + Send + Sync + Default;
}

/// A configuration holder. Any type that wants to use the config must implement methods for config holder.
pub struct ConfigHolder<AC: AcceptsConfig>(Arc<AC::Cfg>);
impl<AC: AcceptsConfig> Clone for ConfigHolder<AC> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<AC: AcceptsConfig> std::ops::Deref for ConfigHolder<AC> {
    type Target = AC::Cfg;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

macro_rules! acfg {
    ($(#[doc = $doc:expr])? $ac:ident : $cfg:ty) => {
        $(#[doc = $doc])?
        /// This empty enum type should not be used without [`ConfigHolder`].
        pub enum $ac {}
        impl crate::cfg::AcceptsConfig for $ac {
            type Cfg = $cfg;
        }
    };
}
pub(crate) use acfg;