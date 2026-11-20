-- Look and feel configuration - Inerium aesthetic

local border_active_1 = INERIUM_BORDER_ACTIVE or INERIUM_DRAGON_GRAY or INERIUM_BORDER or "rgba(727169ff)"
local border_active_2 = INERIUM_BORDER_ACTIVE_SECONDARY or border_active_1
local border_inactive = INERIUM_BORDER or "rgba(454444cc)"
local primary = INERIUM_ACCENT_PRIMARY or INERIUM_BLUE or "rgba(7fb4caff)"
local purple = INERIUM_PURPLE or "rgba(938aa9ff)"
local shadow_color = INERIUM_SHADOW or "rgba(18, 16, 16, 0.65)"

hl.config({
    general = {
        gaps_in = 4,
        gaps_out = 4,
        border_size = 1,
        extend_border_grab_area = 10,
        resize_on_border = true,
        col = {
            active_border = {
                colors = { border_active_2, border_active_1 },
                angle = 45,
            },
            inactive_border = border_inactive,
        },
    },
    group = {
        col = {
            border_active = primary,
            border_inactive = border_inactive,
            border_locked_active = purple,
            border_locked_inactive = border_inactive,
        },
        groupbar = {
            col = {
                active = border_active_2,
                inactive = border_inactive,
                locked_active = purple,
                locked_inactive = border_inactive,
            },
        },
    },
    decoration = {
        dim_special = 0.3,
        rounding = 10,
        active_opacity = 0.96,
        inactive_opacity = 0.88,
        fullscreen_opacity = 1.0,
        blur = {
            enabled = true,
            size = 6,
            passes = 3,
            new_optimizations = true,
            xray = false,
            special = true,
        },
        shadow = {
            enabled = true,
            range = 14,
            render_power = 3,
            color = shadow_color,
        },
    },
})
