---
version: alpha
name: MonoTools-design-system
属于: A dark-canvas desktop productivity system that merges MonoTools' existing neutral token architecture with Raycast-inspired design language — pure near-black canvas, frosted-glass surfaces, Inter typography, and restrained accent usage. Elevation is built from a surface-color ladder plus backdrop-blur glass panels, never from drop shadows. The system supports both light and dark themes through CSS custom properties.
description: |
  MonoTools' design system blends its existing neutral-grey token vocabulary (--ww-*) with Raycast's command-palette-inspired dark-mode patterns. The canvas is a near-black (#09090b in dark, #eaeaec in light) with a 5-step surface ladder (canvas → rail → panel → content → elevated). Depth is achieved through subtle borders, glass-blur panels with backdrop-filter, and controlled shadow elevation. Typography uses Inter with optional ss03 stylistic set. Accent color is used sparingly for interactive highlights. Components include frosted-glass dialogs, side-nav panels, pop-tips, toast notifications, and command-palette-style rows — all inheriting their theme from CSS custom properties.

colors:
  # === SURFACE LADDER ===
  canvas:
    light: "#eaeaec"
    dark: "#09090b"
    note: "Page-level background. The dominant surface across every page."
  rail:
    light: "#e2e2e5"
    dark: "#0f0f12"
    note: "Sidebar / secondary navigation rail"
  panel:
    light: "#eeeeef"
    dark: "#131316"
    note: "Panel background, one notch above rail"
  content:
    light: "#ffffff"
    dark: "#18181b"
    note: "Primary content surface / card background"
  elevated:
    light: "#ffffff"
    dark: "#1f1f23"
    note: "Elevated panels, dialogs, modals"
  inset:
    light: "#f5f5f6"
    dark: "#101012"
    note: "Recessed fields, nested containers, input backgrounds"

  # === TEXT ===
  ink:
    light: "#121214"
    dark: "#f4f4f5"
    note: "Primary text — near-black on light, near-white on dark"
  ink-muted:
    light: "#5a5a62"
    dark: "#a1a1aa"
    note: "Secondary text, metadata, captions"
  ink-faint:
    light: "#888890"
    dark: "#71717a"
    note: "Low-emphasis utility text, disabled labels"
  accent:
    light: "#3a3a42"
    dark: "#e4e4e7"
    note: "Interactive highlight, selected state, checkbox fill"
  accent-hover:
    light: "#242428"
    dark: "#fafafa"
    note: "Hover state for accent interactive elements"
  accent-soft:
    light: "#ececee"
    dark: "#27272a"
    note: "Tinted background for badges, active pills"

  # === SEMANTIC ===
  warn:
    light: "#8a6b38"
    dark: "#c9a227"
    note: "Warning indicator text"
  warn-soft:
    light: "#f3efe6"
    dark: "rgb(201 162 39 / 0.14)"
    note: "Warning tinted background"
  danger-text:
    light: "#b91c1c"
    dark: "#f87171"
    note: "Destructive action text"
  toast-success:
    light: "#2d8a5e"
    dark: "#3d9a6a"
    note: "Success toast icon color"
  toast-error:
    light: "#c45c5c"
    dark: "#d07070"
    note: "Error toast icon color"
  toast-info:
    light: "#4a7eb8"
    dark: "#5a8fc4"
    note: "Info toast icon color"
  toast-warn:
    light: "var(--ww-warn)"
    dark: "var(--ww-warn)"
    note: "Warning toast icon color (shared)"

  # === BORDERS ===
  border-subtle:
    light: "rgb(18 18 22 / 0.06)"
    dark: "rgb(255 255 255 / 0.08)"
    note: "Primary 1px border for cards, inputs, panels"
  border-faint:
    light: "rgb(18 18 22 / 0.05)"
    dark: "rgb(255 255 255 / 0.05)"
    note: "Ultra-faint border for nested elements"
  glass-border:
    light: "rgb(18 18 22 / 0.08)"
    dark: "rgb(255 255 255 / 0.1)"
    note: "Glass panel border"
  glass-border-dark:
    light: "rgb(255 255 255 / 0.28)"
    dark: "rgb(255 255 255 / 0.22)"
    note: "Stronger glass border for dark glass surfaces"

  # === GLASS ===
  glass-bg:
    light: "rgb(255 255 255 / 0.68)"
    dark: "rgb(30 30 34 / 0.8)"
    note: "Glass panel background (semi-transparent)"
  glass-bg-soft:
    light: "rgb(255 255 255 / 0.78)"
    dark: "rgb(36 36 40 / 0.85)"
    note: "Soft glass background for dialogs, popups"
  glass-hover-bg:
    dark: "rgb(30 30 34 / 0.9)"
    note: "Hover state for glass surfaces"

  # === LIST STATES ===
  list-hover-bg:
    light: "#ececee"
    dark: "#252528"
    note: "Row hover background"
  list-selected-bg:
    light: "#e0e0e4"
    dark: "#2e2e32"
    note: "Selected row background"
  list-selected-ring:
    light: "rgb(18 18 22 / 0.14)"
    dark: "rgb(255 255 255 / 0.12)"
    note: "Selected item focus ring"
  list-hover-ring:
    light: "rgb(18 18 22 / 0.08)"
    dark: "rgb(255 255 255 / 0.06)"
    note: "Hover item focus ring"
  list-selected-accent:
    light: "#3a3a42"
    dark: "#e4e4e7"
    note: "Selected checkbox / toggle fill"

  # === MISC ===
  tag-bg:
    light: "#e4e4e8"
    dark: "#2c2c30"
    note: "Tag / chip background"
  tag-fg: "var(--ww-ink-muted)"
  tag-border:
    light: "rgb(18 18 22 / 0.08)"
    dark: "rgb(255 255 255 / 0.1)"
  switch-track:
    light: "#d8d8dc"
    dark: "#3f3f46"
  switch-track-hover:
    light: "#c8c8ce"
    dark: "#52525b"
  switch-track-on: "var(--ww-list-selected-accent)"
  switch-track-on-hover: "var(--ww-accent-hover)"
  switch-thumb:
    light: "#ffffff"
    dark: "#a1a1aa"
  switch-thumb-on:
    light: "#ffffff"
    dark: "#18181b"
  thumb-add-dash:
    light: "rgb(18 18 22 / 0.24)"
    dark: "rgb(255 255 255 / 0.26)"
  warn-soft:
    light: "#f3efe6"
    dark: "rgb(201 162 39 / 0.14)"
  field-bg:
    light: "rgb(18 18 22 / 0.04)"
    dark: "rgb(255 255 255 / 0.06)"
  field-bg-focus:
    light: "rgb(18 18 22 / 0.06)"
    dark: "rgb(255 255 255 / 0.09)"
  overlay-scrim:
    light: "rgb(18 18 22 / 0.38)"
    dark: "rgb(0 0 0 / 0.52)"
  surface: "#ffffff"
  surface-float:
    light: "rgb(255 255 255 / 0.9)"
    dark: "rgb(30 30 34 / 0.92)"
  surface-raised: "var(--ww-inset)"
  danger-text:
    light: "#b91c1c"
    dark: "#f87171"
  on-media-fg: "rgb(255 255 255 / 0.95)"
  on-media-fg-muted: "rgb(255 255 255 / 0.72)"
  grid-dot:
    light: "rgb(18 18 22 / 0.055)"
    dark: "rgb(255 255 255 / 0.04)"

typography:
  fontFamily: "Inter, PingFang SC, Source Han Sans SC, system-ui, sans-serif"
  fontFeatureSettings: "\"calt\", \"kern\", \"liga\", \"ss03\""
  baseSize: "0.8125rem"
  baseLineHeight: "1.55"
  antiAlias: "antialiased"
  hierarchy:
    display-xl:
      fontSize: "clamp(1.75rem, 4.5vw, 2.75rem)"
      fontWeight: 500
      lineHeight: 1.08
      letterSpacing: "-0.04em"
      use: "Editorial hero title, page hero headline"
    display-lg:
      fontSize: "clamp(1.625rem, 3.2vw, 2.125rem)"
      fontWeight: 500
      lineHeight: 1.12
      letterSpacing: "-0.035em"
      use: "Page header title, section hero"
    heading-xl:
      fontSize: "1.375rem"
      fontWeight: 500
      lineHeight: 1.2
      letterSpacing: "-0.02em"
      use: "Cloud-abode page title, major section heading"
    heading-lg:
      fontSize: "1.0625rem"
      fontWeight: 600
      lineHeight: 1.3
      use: "Dialog header, card group title"
    heading-md:
      fontSize: "0.9375rem"
      fontWeight: 500
      lineHeight: 1.4
      use: "In-card heading, feature label"
    heading-sm:
      fontSize: "0.8125rem"
      fontWeight: 600
      lineHeight: 1.4
      use: "Section label, uppercase kicker"
    body-lg:
      fontSize: "0.9375rem"
      fontWeight: 400
      lineHeight: 1.65
      use: "Hero subtitle, pricing description"
    body-md:
      fontSize: "0.8125rem"
      fontWeight: 400
      lineHeight: 1.55
      use: "Default body text (global base)"
    body-strong:
      fontSize: "0.8125rem"
      fontWeight: 500
      lineHeight: 1.4
      use: "Inline emphasis, nav link"
    body-sm:
      fontSize: "0.75rem"
      fontWeight: 400
      lineHeight: 1.45
      use: "Card description, secondary copy"
    body-sm-strong:
      fontSize: "0.75rem"
      fontWeight: 600
      lineHeight: 1.35
      use: "In-card label, table header"
    caption-md:
      fontSize: "0.6875rem"
      fontWeight: 500
      lineHeight: 1.4
      letterSpacing: "0.04em"
      use: "Caption, metadata, badge label"
    caption-sm:
      fontSize: "0.625rem"
      fontWeight: 500
      lineHeight: 1.3
      letterSpacing: "0.04em"
      use: "Smallest utility text, provider badge"
    button-md:
      fontSize: "0.8125rem"
      fontWeight: 600
      lineHeight: 1
      use: "Standard button label"
  principles: |
    The hierarchy uses a tight line-height ladder (1.08–1.65). Negative letter-spacing on display/heading tiers (-0.02em to -0.04em) gives editorial density; captions use positive tracking (0.04em) for airiness at small sizes. The global base is 0.8125rem / 1.55 — reach for 0.75rem for secondary copy and clamp() display sizes for hero moments.

rounded:
  none: "0px"
  xs: "0.25rem"
  sm: "0.375rem"
  md: "0.5rem"
  lg: "0.75rem"
  xl: "1rem"
  "2xl": "1.25rem"
  "3xl": "1.5rem"
  full: "999px"
  note: |
    The radius vocabulary clusters between 4px and 16px. Most chrome uses 6–10px (sm–lg).
    Full-radius pills (999px) are reserved for tabs, chips, and toggles.

spacing:
  xxs: "0.125rem"
  xs: "0.25rem"
  sm: "0.375rem"
  md: "0.5rem"
  lg: "0.625rem"
  xl: "0.75rem"
  xxl: "1rem"
  "2xl": "1.25rem"
  "3xl": "1.5rem"
  "4xl": "2rem"
  section: "clamp(1.25rem, 3vw, 2rem)"
  note: |
    Base unit is 4px with 2px steps for tight inline gaps. Section spacing uses clamp() for fluid responsiveness.
    Page padding: var(--ww-page-padding) = 1.125rem (18px).

elevation:
  note: |
    Elevation is built from the surface-color ladder + glass blur + controlled shadows. No pure drop-shadow-only elevation.
  levels:
    flat:
      treatment: "No border, no shadow"
      use: "Canvas-on-canvas blocks, hero text, footer body"
    hairline:
      treatment: "1px solid var(--ww-border-subtle)"
      use: "Every card on content surface, input borders, panel edges"
    hairline-strong:
      treatment: "1px solid var(--ww-border-subtle) or glass-border"
      use: "Selected items, stronger dividers, glass panel edges"
    glass:
      treatment: "backdrop-filter blur(24px) + semi-transparent bg"
      use: "Dialogs, menus, popups, toasts, sticky headers"
    shadow-card:
      treatment: "box-shadow: var(--ww-shadow-card)"
      use: "Hover elevation on cards, side-nav panels"
    shadow-soft:
      treatment: "box-shadow: var(--ww-shadow-soft)"
      use: "Floating panels, glass surfaces"
  shadows:
    shadow-card: "0 8px 24px -6px rgb(18 18 22 / 0.08), 0 3px 10px -3px rgb(18 18 22 / 0.05)"
    shadow-soft: "0 14px 36px -8px rgb(18 18 22 / 0.1), 0 6px 16px -6px rgb(18 18 22 / 0.06)"
    shadow-hover: "0 40px 72px -14px rgb(18 18 22 / 0.12), 0 22px 48px -12px rgb(18 18 22 / 0.08)"
  dark-shadows:
    shadow-card: "0 8px 24px -6px rgb(0 0 0 / 0.4), 0 0 0 1px rgb(255 255 255 / 0.04)"
    shadow-soft: "0 14px 36px -8px rgb(0 0 0 / 0.45), 0 6px 16px -6px rgb(0 0 0 / 0.35)"
    shadow-hover: "0 40px 72px -14px rgb(0 0 0 / 0.55), 0 22px 48px -12px rgb(0 0 0 / 0.4)"

motion:
  ease-out: "cubic-bezier(0.22, 1, 0.36, 1)"
  ease-out-slow: "cubic-bezier(0.16, 1, 0.3, 1)"
  duration-fast: "0.22s"
  duration: "0.36s"
  duration-slow: "0.48s"
  note: |
    All transitions use the ease-out family. Fast for micro-interactions (hover, focus),
    default for panel transitions, slow for reveal/entrance animations.

layout:
  content-max: "68rem"
  page-padding: "1.125rem"
  sidebar-width: "5.25rem"
  subpanel-width: "15.5rem"
  titlebar-height: "1.75rem"
  viewer-inset: "0.75rem"
  hero-height: "min(26rem, 48vh)"
  toast-inset: "1.5rem"
  toast-min-width: "14rem"
  toast-max-width: "20rem"
  grid-dot-size: "20px"
  note: |
    Content is capped at 68rem (~1088px) with 18px page padding. Sidebar is a compact 84px rail.
    Toast notifications sit in the bottom-right with 24px inset.

z-index:
  image-viewer: 9999
  command-palette: 10000
  popover: 10045
  context-menu: 10050
  dialog: 10100
  toast: 120
  pop-tip: 120

components:
  # === BUTTONS ===
  button-primary:
    description: "Primary action button — accent filled"
    background: "var(--ww-accent)"
    textColor: "var(--ww-content)"
    typography: "{typography.button-md}"
    rounded: "{rounded.md}"
    padding: "0.45rem 0.9rem"
    border: "1px solid var(--ww-accent)"
    transition: "background var(--ww-duration-fast) var(--ww-ease-out), transform var(--ww-duration-fast) var(--ww-ease-out)"
    hover:
      background: "var(--ww-accent-hover)"
      borderColor: "var(--ww-accent-hover)"
    active:
      transform: "scale(0.98)"
    use: "Primary CTA, form submit, confirmation actions"
  button-secondary:
    description: "Secondary button — outlined"
    background: "var(--ww-content)"
    textColor: "var(--ww-ink)"
    typography: "{typography.button-md}"
    rounded: "{rounded.md}"
    padding: "0.45rem 0.9rem"
    border: "1px solid var(--ww-border-subtle)"
    transition: "background var(--ww-duration-fast) var(--ww-ease-out), transform var(--ww-duration-fast) var(--ww-ease-out)"
    hover:
      background: "var(--ww-list-hover-bg)"
    active:
      transform: "scale(0.98)"
    use: "Secondary actions, cancel, filters"
  button-text:
    description: "Text-only button — no border or fill"
    background: "transparent"
    textColor: "var(--ww-accent)"
    typography: "{typography.button-md}"
    rounded: "{rounded.md}"
    border: "none"
    hover:
      opacity: 0.75
    use: "Low-emphasis actions, links-as-buttons, toast actions"
  button-disabled:
    description: "Disabled state"
    opacity: 0.45
    cursor: "not-allowed"
  glass-button:
    description: "Frosted-glass floating action button"
    background: "rgb(255 255 255 / 0.12)"
    border: "1px solid rgb(255 255 255 / 0.16)"
    textColor: "#fff"
    backdropFilter: "blur(20px) saturate(1.25)"
    rounded: "{rounded.lg}"
    width: "2rem"
    height: "2rem"
    hover:
      background: "rgb(255 255 255 / 0.2)"
      borderColor: "rgb(255 255 255 / 0.28)"
    dark-glass:
      background: "var(--ww-glass-bg)"
      border: "1px solid var(--ww-glass-border)"
      backdropFilter: "blur(var(--ww-blur-glass)) saturate(1.35)"
    use: "Floating actions on media surfaces, hero overlays"

  # === INPUTS & FORMS ===
  text-input:
    description: "Standard text input field"
    background: "var(--ww-inset)"
    textColor: "var(--ww-ink)"
    typography: "{typography.body-md}"
    rounded: "{rounded.md}"
    padding: "0.45rem 0.75rem"
    border: "1px solid var(--ww-border-subtle)"
    placeholderColor: "var(--ww-ink-faint)"
    focus:
      borderColor: "var(--ww-list-selected-ring)"
      boxShadow: "0 0 0 2px var(--ww-list-hover-ring)"
    use: "Standard form input, search field, filter input"
  select-field:
    description: "Select / dropdown field (matches text-input chrome)"
    background: "var(--ww-inset)"
    textColor: "var(--ww-ink)"
    rounded: "{rounded.md}"
    border: "1px solid var(--ww-border-subtle)"
  checkbox:
    description: "Custom checkbox"
    width: "1.125rem"
    height: "1.125rem"
    border: "1px solid var(--ww-border-subtle)"
    rounded: "{rounded.xs}"
    background: "var(--ww-inset)"
    checked:
      background: "var(--ww-accent)"
      borderColor: "var(--ww-accent)"
      checkColor: "var(--ww-canvas)"
    hover:
      borderColor: "rgb(255 255 255 / 0.12)"
      background: "var(--ww-list-hover-bg)"
    use: "Settings toggles, selection lists"

  # === CARDS & PANELS ===
  card:
    description: "Standard content card"
    background: "var(--ww-content)"
    border: "1px solid var(--ww-border-subtle)"
    rounded: "{rounded.lg}"
    padding: "1rem 1.125rem"
    shadow: "var(--ww-shadow-card)"
    interactive-hover:
      borderColor: "var(--ww-border-faint)"
      boxShadow: "0 8px 24px -12px rgb(18 18 22 / 0.12)"
      transform: "translateY(-1px)"
    use: "Content grouping, feature cards, list items"
  glass-panel:
    description: "Frosted-glass floating panel"
    background: "var(--ww-glass-bg)"
    border: "1px solid var(--ww-glass-border)"
    rounded: "{rounded.xl}"
    backdropFilter: "blur(var(--ww-blur-glass)) saturate(1.5)"
    shadow: "var(--ww-shadow-soft)"
    use: "Dialogs, context menus, popovers, floating toolbars"
  glass-dialog:
    description: "Modal dialog with glass treatment"
    background: "var(--ww-glass-bg-soft)"
    border: "1px solid var(--ww-glass-border)"
    rounded: "{rounded.xl}"
    backdropFilter: "blur(var(--ww-dialog-panel-blur)) saturate(1.45)"
    shadow: "var(--ww-menu-shadow)"
    mask:
      background: "var(--ww-overlay-scrim)"
      backdropFilter: "blur(var(--ww-dialog-mask-blur)) saturate(1.25)"
    use: "Confirmation dialogs, share dialogs, settings panels"
  frosted-dialog:
    description: "Strong frosted dialog (share card, image preview)"
    background: "rgb(255 255 255 / 0.38)"
    backdropFilter: "blur(var(--ww-share-dialog-panel-blur)) saturate(1.65)"
    boxShadow: "var(--ww-menu-shadow), inset 0 1px 0 rgb(255 255 255 / 0.45)"
    dark:
      background: "rgb(28 28 32 / 0.34)"
      boxShadow: "var(--ww-menu-shadow), inset 0 1px 0 rgb(255 255 255 / 0.06)"
    use: "High-visibility modals, share-card customization"
  side-nav-panel:
    description: "Compact sidebar panel container"
    background: "var(--ww-elevated)"
    border: "1px solid var(--ww-border-subtle)"
    rounded: "{rounded.lg}"
    padding: "0.375rem"
    hover:
      boxShadow: "var(--ww-shadow-card)"
    use: "Module navigation sidebar, settings rail"

  # === NAVIGATION ===
  pill-tab:
    description: "Pill-shaped segmented control / tab chip"
    default:
      background: "transparent"
      textColor: "var(--ww-ink-muted)"
      border: "none"
      rounded: "{rounded.full}"
      padding: "0.4375rem 0.9375rem"
    active:
      background: "var(--ww-content)"
      textColor: "var(--ww-ink)"
      boxShadow: "0 1px 4px color-mix(in srgb, black 8%, transparent)"
    hover:
      color: "var(--ww-ink)"
    use: "Tab navigation, segmented filters, module switchers"
  tab-bar:
    description: "Shell-level sticky tab bar"
    background: "color-mix(in srgb, var(--ww-inset) 88%, transparent)"
    border: "1px solid var(--ww-glass-border)"
    rounded: "{rounded.full}"
    active:
      background: "var(--ww-content)"
      color: "var(--ww-ink)"
      boxShadow: "inset 0 0 0 1px var(--ww-border-subtle)"
    use: "Top-level module tabs (CloudAbode, Music, Library, etc.)"

  # === DATA DISPLAY ===
  product-card:
    description: "Cloud-abode product card with glass blur"
    rounded: "1.125rem"
    border: "1px solid var(--ww-border-subtle)"
    media-aspect: "4 / 5"
    body-padding: "1rem 1.05rem 1.1rem"
    hover:
      borderColor: "var(--ww-border-faint)"
      mediaTransform: "scale(1.01)"
    badge:
      background: "color-mix(in srgb, var(--ww-content) 88%, transparent)"
      backdropFilter: "blur(6px)"
    use: "Vehicle / product grid cards in CloudAbode"
  mood-card:
    description: "Square mood/atmosphere card"
    aspect-ratio: "1"
    rounded: "{rounded.lg}"
    border: "1px solid var(--ww-glass-border)"
    background: "var(--ww-music-glass-panel)"
    shadow: "var(--ww-shadow-card)"
    hover:
      transform: "translateY(-2px)"
      borderColor: "color-mix(in srgb, var(--ww-ink) 12%, var(--ww-border-subtle))"
      boxShadow: "0 8px 18px color-mix(in srgb, black 16%, transparent)"
    use: "Music mood/atmosphere tiles"
  stat-tile:
    description: "Statistics value tile"
    padding: "1rem 1.1rem"
    rounded: "{rounded.xl}"
    value:
      fontSize: "1.25rem"
      fontWeight: 500
      letterSpacing: "-0.02em"
      fontVariantNumeric: "tabular-nums"
    label:
      fontSize: "0.6875rem"
      fontWeight: 600
      letterSpacing: "0.05em"
      textTransform: "uppercase"
      color: "var(--ww-ink-faint)"
    use: "Dashboard stats, wallet balances"

  # === OVERLAYS & FEEDBACK ===
  toast:
    description: "Bottom-right toast notification"
    background: "var(--ww-glass-bg)"
    border: "1px solid var(--ww-glass-border)"
    rounded: "{rounded.lg}"
    backdropFilter: "blur(var(--ww-menu-blur))"
    shadow: "var(--ww-menu-shadow)"
    minWidth: "var(--ww-toast-min-width)"
    maxWidth: "var(--ww-toast-max-width)"
    icon:
      success: "var(--ww-toast-success)"
      error: "var(--ww-toast-error)"
      info: "var(--ww-toast-info)"
      warn: "var(--ww-toast-warn)"
    use: "System feedback, async operation results"
  pop-tip:
    description: "Top-center capsule pop tip (lightweight toast)"
    background: "var(--ww-glass-bg-soft)"
    border: "1px solid var(--ww-glass-border)"
    rounded: "{rounded.full}"
    padding: "0.5rem 1rem"
    backdropFilter: "blur(16px) saturate(1.35)"
    shadow: "var(--ww-shadow-soft)"
    fontSize: "0.8125rem"
    fontWeight: 500
    use: "Copy feedback, quick confirmation, transient status"
  dialog-footer:
    description: "Dialog action button row"
    display: "flex"
    alignItems: "center"
    justifyContent: "flex-end"
    flexWrap: "wrap"
    gap: "var(--ww-dialog-footer-gap)"
    padding: "0.5625rem 1rem 0.875rem"
    cancel:
      color: "var(--ww-dialog-footer-cancel-fg)"
      background: "var(--ww-dialog-footer-cancel-bg)"
      border: "transparent"
      hover:
        background: "var(--ww-dialog-footer-cancel-bg-hover)"
        color: "var(--ww-ink)"
    use: "Dialog CTA row, confirmation actions"

  # === MISCELLANEOUS ===
  page-header:
    description: "Page title bar with optional subtitle and actions"
    padding: "calc(var(--ww-titlebar-height) + 0.625rem) var(--ww-page-padding) 0.875rem"
    borderBottom: "1px solid var(--ww-border-subtle)"
    title:
      fontSize: "1rem"
      fontWeight: 600
      letterSpacing: "-0.02em"
    subtitle:
      fontSize: "0.8125rem"
      color: "var(--ww-ink-muted)"
    use: "Top of every page view"
  cover-image:
    description: "Image with placeholder fallback"
    placeholder:
      background: "var(--ww-inset)"
      color: "var(--ww-ink-faint)"
      fontSize: "0.6875rem"
    use: "Album art, product images, note attachments"
  icon:
    description: "Lucide icon component (WwIcon)"
    sizes:
      xs: 14
      sm: 16
      md: 18
      lg: 20
    strokeWidth: 1.5
    spin:
      animation: "ww-icon-spin 0.85s linear infinite"
    use: "All iconography throughout the app"

glass-system:
  blur-strong: "40px"
  blur-glass: "24px"
  blur-menu: "32px"
  blur-dialog-mask: "28px"
  blur-dialog-panel: "48px"
  saturation: "1.35–1.75"
  note: |
    Glass is the primary depth mechanism. All glass surfaces use backdrop-filter with
    blur + saturation boost. The system provides three tiers:
    - Light glass (blur-glass: 24px): panels, sidebars
    - Strong glass (blur-strong: 40px): floating panels, overlays
    - Dialog glass (blur-dialog-panel: 48px): modals and confirmations

theming:
  modes:
    - light
    - dark
  switch: "[data-theme='dark'] attribute on <html>"
  strategy: |
    All colors are CSS custom properties on :root (light defaults).
    [data-theme='dark'] overrides only the semantic tokens.
    Business styles reference var(--ww-*) exclusively — never hard-coded colors.
  dark-overrides-note: |
    Dark mode deepens the surface ladder (#09090b → #0f0f12 → #131316 → #18181b → #1f1f23),
    shifts ink to near-white, and increases shadow opacity for depth on dark backgrounds.
    Glass backgrounds gain higher opacity (0.8–0.85 vs 0.68–0.78 in light) to maintain
    readability against dark surfaces.

spacing-system:
  base-unit: "4px"
  inline-steps: "2px / 4px / 6px / 8px"
  page-padding: "18px (1.125rem)"
  section-gap: "clamp(20px, 3vw, 32px)"
  card-internal-padding: "16–24px"
  grid-gutter: "10–16px"
  note: |
    Spacing flows from a 4px base with common steps at 8/12/16/24/32px.
    Cards use 16–24px internal padding. Section gaps are fluid via clamp().

raycast-influences:
  note: |
    Raycast design principles integrated into MonoTools:
    - Dark-canvas-first philosophy with near-black backgrounds
    - Hairline 1px borders (#242728 in Raycast → border-subtle in MonoTools)
    - Surface-color elevation ladder (no drop-shadow-only elevation)
    - Inter font with ss03 stylistic set for signature g-glyph
    - Command-palette-style rows for search results and quick actions
    - Frosted-glass overlays for dialogs and menus
    - Restrained accent color — used sparingly for interactive highlights
    - Compact, dense information layout suited for power users
  differences:
    - MonoTools supports both light and dark themes (Raycast is dark-only)
    - MonoTools uses CSS custom properties (--ww-*) for full theme switching
    - MonoTools incorporates glass-blur as primary depth mechanism
    - Sidebar rail navigation (5.25rem) is unique to MonoTools desktop layout

iconography:
  system: "Lucide (via WwIcon wrapper component)"
  strokeWidth: 1.5
  sizes:
    xs: 14px
    sm: 16px
    md: 18px (default)
    lg: 20px
  spin: "0.85s linear infinite rotation"
  filled: "currentColor fill (default: none)"
  custom: "SVG files in /icons/ for domain-specific glyphs"
  note: "All icons inherit currentColor from parent text color. No icon-specific colors except in semantic contexts (toasts, status)."

known-gaps:
  - "Hover states documented per-component but not exhaustively — Raycast-style rich hover on command-palette rows not fully captured"
  - "Light mode shadows are subtle; dark mode shadows use higher opacity for visibility — system relies more on glass blur than shadows for elevation"
  - "Music module has its own sub-token layer (--ww-music-*) that extends the core system — documented separately in music-shared.css"
  - "Cloud-abode module uses a distinct editorial/velvety visual language on top of the core tokens"
  - "Form validation states beyond the warn-tinted border are minimal"
  - "No brand gradient / hero stripe system — MonoTools uses neutral accents instead of Raycast's red gradient band"
  - "Command palette mockup component (Raycast-style hero) not yet implemented as a standalone component"
