//! Fallback CSS variables and alias tokens for Kanagawa themes.

pub const DRAGON_VARS: &str = r#"
@define-color cc_bg               rgba(24, 22, 22, 0.97);
@define-color cc_surface          #221f1f;
@define-color cc_surface_hover    #2d2a2a;
@define-color cc_surface_active   #363434;
@define-color cc_line             rgba(197, 201, 197, 0.10);
@define-color cc_line_light       rgba(197, 201, 197, 0.20);
@define-color cc_fg               #c5c9c5;
@define-color cc_fg_bright        #e6e2cc;
@define-color cc_fg_muted         #737c73;
@define-color cc_fg_subtle        #a6a69c;
@define-color cc_fg_on_accent     #181616;

@define-color cc_green            #87a987;
@define-color cc_green_bg         rgba(135, 169, 135, 0.16);
@define-color cc_green_border     rgba(135, 169, 135, 0.35);

@define-color cc_blue             #8ba4b0;
@define-color cc_blue_bg          rgba(139, 164, 176, 0.16);
@define-color cc_blue_border      rgba(139, 164, 176, 0.38);

@define-color cc_amber            #c4b28a;
@define-color cc_amber_bg         rgba(196, 178, 138, 0.15);
@define-color cc_amber_border     rgba(196, 178, 138, 0.38);

@define-color cc_red              #c4746e;
@define-color cc_red_bg           rgba(196, 116, 110, 0.15);
@define-color cc_red_border       rgba(196, 116, 110, 0.38);

@define-color cc_aqua             #8ea4a2;
@define-color cc_aqua_bg          rgba(142, 164, 162, 0.16);
@define-color cc_aqua_border      rgba(142, 164, 162, 0.38);

@define-color cc_violet           #8992a7;
@define-color cc_violet_bg        rgba(137, 146, 167, 0.16);
@define-color cc_violet_border    rgba(137, 146, 167, 0.38);

@define-color cc_shadow           rgba(0, 0, 0, 0.85);
@define-color cc_modal_backdrop   rgba(18, 16, 16, 0.48);
@define-color cc_slider_trough    rgba(197, 201, 197, 0.10);
@define-color cc_slider_thumb     #e6e2cc;
@define-color cc_orange           #b6927b;
@define-color cc_primary          #8ba4b0;
@define-color cc_primary_bg       rgba(139, 164, 176, 0.16);
@define-color cc_primary_border   rgba(139, 164, 176, 0.35);
@define-color cc_slider_fill      #87a987;
@define-color cc_slider_fill_secondary #c4b28a;
"#;

pub const LOTUS_VARS: &str = r#"
@define-color cc_bg               rgba(242, 236, 188, 0.98);
@define-color cc_surface          #e5dfc5;
@define-color cc_surface_hover    #ded7bc;
@define-color cc_surface_active   #d5cdb0;
@define-color cc_line             rgba(84, 84, 100, 0.14);
@define-color cc_line_light       rgba(84, 84, 100, 0.24);
@define-color cc_fg               #545464;
@define-color cc_fg_bright        #43436c;
@define-color cc_fg_muted         #8a8980;
@define-color cc_fg_subtle        #716e61;
@define-color cc_fg_on_accent     #f2ecbc;

@define-color cc_green            #6f894e;
@define-color cc_green_bg         rgba(111, 137, 78, 0.18);
@define-color cc_green_border     rgba(111, 137, 78, 0.40);

@define-color cc_blue             #4d699b;
@define-color cc_blue_bg          rgba(77, 105, 155, 0.18);
@define-color cc_blue_border      rgba(77, 105, 155, 0.40);

@define-color cc_amber            #cc6d00;
@define-color cc_amber_bg         rgba(204, 109, 0, 0.18);
@define-color cc_amber_border     rgba(204, 109, 0, 0.40);

@define-color cc_red              #c84053;
@define-color cc_red_bg           rgba(200, 64, 83, 0.18);
@define-color cc_red_border       rgba(200, 64, 83, 0.40);

@define-color cc_aqua             #597b75;
@define-color cc_aqua_bg          rgba(89, 123, 117, 0.18);
@define-color cc_aqua_border      rgba(89, 123, 117, 0.40);

@define-color cc_violet           #8a7ea8;
@define-color cc_violet_bg        rgba(138, 126, 168, 0.18);
@define-color cc_violet_border    rgba(138, 126, 168, 0.40);

@define-color cc_shadow           rgba(84, 84, 100, 0.18);
@define-color cc_modal_backdrop   rgba(84, 84, 100, 0.35);
@define-color cc_slider_trough    rgba(84, 84, 100, 0.15);
@define-color cc_slider_thumb     #43436c;
@define-color cc_orange           #e98a00;
@define-color cc_primary          #4d699b;
@define-color cc_primary_bg       rgba(77, 105, 155, 0.18);
@define-color cc_primary_border   rgba(77, 105, 155, 0.40);
@define-color cc_slider_fill      #6f894e;
@define-color cc_slider_fill_secondary #cc6d00;
"#;

pub const THEME_ALIASES: &str = r#"
@define-color bg          @cc_bg;
@define-color surface     @cc_surface;
@define-color surface-alt @cc_surface_active;
@define-color line        @cc_line;
@define-color hover       @cc_surface_hover;
@define-color fg          @cc_fg;
@define-color fg-bright   @cc_fg_bright;
@define-color muted       @cc_fg_muted;
@define-color subtle      @cc_fg_subtle;
@define-color green       @cc_green;
@define-color blue        @cc_blue;
@define-color aqua        @cc_aqua;
@define-color yellow      @cc_amber;
@define-color orange      @cc_orange;
@define-color red         @cc_red;
"#;
