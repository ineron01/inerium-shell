-- Session environment variables are managed declaratively via UWSM (~/.config/uwsm/env)
-- and systemd environment.d (~/.config/environment.d/10-theme.conf).
-- Local hardware-specific GPU/driver variables are loaded below:

pcall(require, "config.environment_local")
