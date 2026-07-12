---
version: beta
name: MonoTools-design-system
属于: A pure-monochrome desktop productivity system inspired by BENTO PRO and Raycast — strictly black/white/grey palette with a subtle warm-white accent (#f5f1e8) reserved for key interactions (selected state, focus rings, indexing indicator, checkbox check fill). 90% of the visual is grayscale. Near-black canvas (#15151a in dark), hairline 1px dividers, Inter typography, frosted-glass surfaces. Elevation is built from a surface-color ladder plus backdrop-blur, never from drop shadows. The system supports both light and dark themes through CSS custom properties.
description: |
  MonoTools' design system is built on a strict monochrome (black/white/grey) palette with a subtle warm-white accent. The canvas is a near-black (#15151a in dark, #f5f5f7 in light) with a 6-step surface ladder (canvas → canvas-elevated → rail → panel → content → elevated → inset). Depth is achieved through hairline 1px dividers and glass-blur panels with backdrop-filter, never colored fills. The accent color #f5f1e8 (warm-white) is used only for the CheckButton's filled checkmark, the indexing spinner background, and selected focus accents — keeping the rest strictly grayscale. Typography uses Inter with uppercase labels at 15px / 0.06em tracking for group headers (significantly larger than typical micro-labels for stronger hierarchy). Components include frosted-glass dialogs, groupbox-style list rows (no card backgrounds, only hairline dividers), and command-palette-style search. The search overlay is 780px wide with capped height (max 600px) for a tighter, launcher-style footprint. The "所有文件" filter panel uses a custom animated CheckButton component with draw-on checkmark animations and warm-white fill on the checked state.

colors:
  # === SURFACE LADDER (纯黑白灰, 无彩色, 略深一档) ===
  canvas:
    light: "#f5f5f7"
    dark: "#15151a"
    note: "Page-level background. The dominant surface across every page. Slightly darker than #18181b for stronger presence on the desktop."
  canvas-elevated:
    light: "#ffffff"
    dark: "#1c1c20"
    note: "Slightly raised canvas for app containers"
  rail:
    light: "#ececef"
    dark: "#19191d"
    note: "Sidebar / secondary navigation rail"
  panel:
    light: "#fafafa"
    dark: "#1d1d21"
    note: "Panel background, one notch above rail"
  content:
    light: "#ffffff"
    dark: "#222227"
    note: "Primary content surface / card background"
  elevated:
    light: "#ffffff"
    dark: "#28282e"
    note: "Elevated panels, dialogs, modals"
  inset:
    light: "#f0f0f3"
    dark: "#131317"
    note: "Recessed fields, nested containers, input backgrounds"

  # === TEXT (灰度阶梯) ===
  text-primary:
    light: "#18181b"
    dark: "#ffffff"
    note: "Primary text — near-black on light, near-white on dark"
  text-secondary:
    light: "#3f3f46"
    dark: "#d4d4d8"
    note: "Secondary text, metadata, captions"
  text-tertiary:
    light: "#71717a"
    dark: "#a1a1aa"
    note: "Tertiary text, group headers, labels"
  text-quaternary:
    light: "#a1a1aa"
    dark: "#71717a"
    note: "Low-emphasis utility text, disabled labels"
  text-muted:
    light: "#d4d4d8"
    dark: "#52525b"
    note: "Faintest text, decorative count badges"

  # === ACCENT (细微点缀: 极淡暖白, 用于关键交互; 整体仍 90% 黑白灰) ===
  accent:
    light: "#18181b"
    dark: "#f5f1e8"
    note: "Single chromatic accent — warm-white (almost-neutral). Used ONLY for: CheckButton filled checkmark, indexing spinner background, focus caret, active filter trigger, focus rings. Everything else is grayscale."
  accent-soft:
    light: "rgba(24, 24, 27, 0.06)"
    dark: "rgba(245, 241, 232, 0.08)"
    note: "Tinted background for active pill, indexing pill"
  accent-glow:
    light: "rgba(24, 24, 27, 0.12)"
    dark: "rgba(245, 241, 232, 0.18)"
    note: "Subtle drop-shadow glow on selected state and active icon"
  accent-on-accent:
    light: "#ffffff"
    dark: "#15151a"
    note: "Foreground on accent-filled surfaces (checkbox check, button text)"
  accent-hover:
    light: "#27272a"
    dark: "#ffffff"
    note: "Hover state for accent-filled elements (CheckButton checked state)"
  accent-active:
    light: "#3f3f46"
    dark: "#d4ccb8"
    note: "Active/pressed state for accent elements"

  # === SEMANTIC (错误仍保留红色, 其余全部黑白灰) ===
  danger:
    light: "#b91c1c"
    dark: "#f87171"
    note: "Destructive action text (only color used)"
  danger-bg:
    light: "rgba(185, 28, 28, 0.08)"
    dark: "rgba(248, 113, 113, 0.12)"
    note: "Error tinted background"

  # === BORDERS (细微黑白灰) ===
  border-subtle:
    light: "rgba(24, 24, 27, 0.06)"
    dark: "rgba(255, 255, 255, 0.06)"
    note: "Primary 1px hairline divider for list sections, panel edges"
  border-default:
    light: "rgba(24, 24, 27, 0.10)"
    dark: "rgba(255, 255, 255, 0.10)"
    note: "Standard 1px border for inputs, focus rings, active chips"
  border-hover:
    light: "rgba(24, 24, 27, 0.18)"
    dark: "rgba(255, 255, 255, 0.18)"
    note: "Hover state border"
  border-active:
    light: "rgba(24, 24, 27, 0.28)"
    dark: "rgba(255, 255, 255, 0.28)"
    note: "Active state border, strong focus ring"

  # === GLASS (略深色高级玻璃) ===
  glass-bg:
    light: "rgba(245, 245, 247, 0.82)"
    dark: "rgba(18, 18, 21, 0.82)"
    note: "Main window glass background (semi-transparent over wallpaper). Bumped from 0.72 → 0.82 for more substantial surface feel."
  glass-bg-soft:
    light: "rgba(255, 255, 255, 0.88)"
    dark: "rgba(24, 24, 28, 0.78)"
    note: "Soft glass for dialogs, sticky headers"
  glass-border:
    light: "rgba(24, 24, 27, 0.07)"
    dark: "rgba(255, 255, 255, 0.07)"
    note: "Glass panel border (1px hairline, slightly subtler than before)"

  # === LIST STATES (更细微, 不喧宾夺主) ===
  list-hover-bg:
    light: "rgba(24, 24, 27, 0.025)"
    dark: "rgba(255, 255, 255, 0.025)"
    note: "Row hover background (very subtle, was 0.04 — reduced for less visual noise)"
  list-selected-bg:
    light: "rgba(24, 24, 27, 0.05)"
    dark: "rgba(255, 255, 255, 0.05)"
    note: "Selected row background (subtle white/dark tint, was 0.08 — reduced per feedback)"
  list-selected-ring:
    light: "rgba(24, 24, 27, 0.10)"
    dark: "rgba(255, 255, 255, 0.10)"
    note: "Selected item border ring (1px, was 0.18 — reduced)"
  list-hover-ring:
    light: "rgba(24, 24, 27, 0.05)"
    dark: "rgba(255, 255, 255, 0.05)"
    note: "Hover item border ring (1px, was 0.10 — reduced)"

  # === MISC ===
  shadow-sm: "0 1px 2px rgb(0 0 0 / 0.32)"
  shadow-md: "0 4px 12px rgb(0 0 0 / 0.42)"
  shadow-lg: "0 8px 24px rgb(0 0 0 / 0.52)"
  shadow-xl: "0 16px 48px rgb(0 0 0 / 0.62)"
  overlay-scrim:
    light: "rgb(18 18 22 / 0.38)"
    dark: "rgb(0 0 0 / 0.52)"

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

# ============================================================
# 搜索页 (Search Page) 设计规范 — 黑白灰极简版 v3
# ============================================================
search-page:
  description: |
    The main search window (invoked via Alt+Space) uses a strict monochrome
    layout with a subtle soft-blue accent reserved for key interactions.
    No category banner tab bar. Content flows as a single grouped list with
    hairline dividers between groups. Each row is a result item with icon
    + title + subtitle + meta. The "所有文件" group has a dropdown multi-select
    filter trigger in its header (replacing the inline chip list). The selected
    row shows a 2px left bar in accent color with a soft glow — instead of a
    bright fill — for a more elegant, less noisy selected state.
  window:
    width: "640px (narrow launcher column)"
    height: "580px initial, auto-resizes to content, max 580px"
    minWidth: "540px"
    minHeight: "320px"
    background: "var(--glass-bg) — rgba(18,18,21,0.82) over wallpaper"
    borderRadius: "0 (full-bleed to screen edges)"
    border: "1px solid var(--glass-border) on top edge only"
    backdropFilter: "blur(40px) saturate(180%)"
    shadow: "0 16px 48px rgb(0 0 0 / 0.55)"

  layout:
    structure: "Column flex: [search-bar 52px] → [grouped-list flex-1 (max 440px)] → [action-bar 30px]"
    padding: "No outer padding — content fills the window edge-to-edge"
    gap: "0 — sections separated by 1px hairline dividers only"
    height-cap: |
      When the natural content height exceeds 440px (input + 440 + status = 522),
      the content area stops growing and a vertical scrollbar appears inside
      the grouped list. The window itself is hard-capped at 600px (Rust clamp)
      to prevent extreme cases.

  group-header:
    titleSize: "15px (was 12px — increased for stronger presence)"
    titleWeight: "600"
    titleLetterSpacing: "0.06em"
    titleCase: "uppercase"
    titleColor: "var(--text-quaternary) (lighter, was text-tertiary)"
    iconSize: "15px (was 11px — matched to title)"
    iconStrokeWidth: "1.7"
    iconColor: "var(--text-quaternary) / opacity 0.8"
    countBadge: "Pill 1px border, text 10.5px, var(--text-muted), padding 0 7px, line-height 17px"
    height: "38px (was 32px — taller for more breathing room)"
    padding: "18px 6px 10px 6px"

  selected-row:
    treatment: "5% white background tint + 2px left bar in warm-white accent (#f5f1e8) with 8px glow"
    iconColor: "warm-white accent (#f5f1e8) with 6px glow"
    badgeColor: "warm-white accent + accent-soft background"
    shortcutHint: "Visible only on hover/active"

  file-kind-filter:
    trigger: "Dropdown button (Filter icon + summary + chevron)"
    summaryDefault: "'全部类型' when all kinds selected"
    summaryPartial: "'已选 N 类' when N kinds selected (N > 2)"
    summarySelected: "Comma-joined labels when N <= 2"
    activeState: "white-5% background + border-subtle + text-primary"
    panel: |
      Floating popover below the trigger, top-right aligned.
      Width: 360px. Background: rgba(20, 20, 24, 0.75) over the page.
      backdrop-filter: blur(40px) saturate(180%) — strong glass treatment.
      Border-radius: var(--radius-xl). Box-shadow: 0 16px 48px / 0 4px 12px.
      Grid layout: 3 columns × N rows, gap 6px.
      Each option: 15×15 CheckButton (square, 5px radius) + label + count badge.
      Footer: '清空' / '全选' actions separated by hairline divider.
    checkButton: |
      Custom CheckButton component (CheckButton.vue):
      - Default: 1px hairline border, transparent background
      - Checked: warm-white (#f5f1e8) fill + dark check icon
      - Hover checked: pure white (#ffffff) fill with subtle glow
      - Animations:
        * Box border→fill: 240ms ease-out
        * Pop-in: cubic-bezier(0.34, 1.56, 0.64, 1), 360ms
        * Check draw: stroke-dasharray 26→0, 360ms ease-out (80ms delay)
        * Check enter: opacity + rotate(-18deg) → 0, 280ms
    animation: "opacity + scale(0.92→1) + translateY(-8px→0), 240ms spring-out (cubic-bezier(0.34, 1.2, 0.64, 1))"
    option: |
      Each option has padding 7px 10px, gap 8px, border-radius 8px.
      Default: transparent background.
      Hover: rgba(255,255,255,0.045) + border-subtle + text-primary.
      Active: rgba(255,255,255,0.06) + border-subtle + text-primary.
    countBadge: |
      Per-option count badge: 18×18px minWidth pill, rgba(255,255,255,0.025) bg.
      On active: rgba(255,255,255,0.08) bg + text-primary.

  # 搜索栏 (Search Bar)
  search-bar:
    height: "52px"
    padding: "0 18px"
    borderBottom: "1px solid var(--border-subtle)"
    background: "transparent"
    icon: "Search 18px / strokeWidth 1.5 / color: var(--text-tertiary)"
    input:
      fontSize: "16px (text-lg)"
      fontWeight: "400"
      color: "var(--text-primary)"
      placeholder: "var(--text-quaternary)"
      caret: "var(--accent) — white in dark, near-black in light"
    clearButton:
      size: "22×22px"
      color: "var(--text-tertiary)"
      hoverBg: "var(--list-hover-bg)"
    logo:
      size: "24×24px"
      filter: "grayscale(100%) opacity(0.5) → hover: grayscale(0%) opacity(1)"
      transition: "0.18s ease-out"

  # 分组列表 (Grouped List) — Raycast 风格, 无卡片化
  grouped-list:
    description: |
      A single scrollable list divided into 6 groups (when not searching):
      固定项目 → 最近访问 → 系统应用 → 命令 → 所有应用 → 所有文件.
      Groups have NO card background, NO border, NO rounded corners.
      They are separated by 1px hairline dividers (var(--border-subtle)).
    padding: "4px 10px 0 10px (horizontal padding 10px from window edge)"

    group:
      separator: "1px solid var(--border-subtle) (top border, first group has none)"
      background: "transparent"
      borderRadius: "0"
      padding: "0"
      margin: "0"

    group-header:
      height: "38px (was 26px — taller for stronger presence)"
      padding: "18px 6px 10px 6px (was 10px 4px 6px 4px)"
      display: "flex / justify-content: space-between / align-items: center"
      icon:
        size: "15px (was 11px — matched to title)"
        strokeWidth: "1.7"
        color: "var(--text-quaternary) / opacity 0.8"
      title:
        fontSize: "15px (was 10.5px — bigger, more prominent)"
        fontWeight: "600"
        letterSpacing: "0.06em"
        textTransform: "uppercase"
        color: "var(--text-quaternary) (lighter, was text-tertiary)"
      count:
        fontSize: "10.5px"
        fontWeight: "500"
        color: "var(--text-muted)"
        fontVariantNumeric: "tabular-nums"
        marginLeft: "4px"
        padding: "0 7px"
        lineHeight: "17px"
        borderRadius: "{rounded.full}"
      chips (only for 所有文件 group):
        display: "flex / justify-content: flex-end / overflow-x: auto"
        gap: "3px"
        chip:
          height: "18px"
          padding: "0 7px"
          borderRadius: "999px (full pill)"
          border: "1px solid var(--border-subtle)"
          background: "transparent"
          color: "var(--text-muted)"
          fontSize: "10.5px / weight 500"
          hover:
            background: "var(--list-hover-bg)"
            borderColor: "var(--border-default)"
            color: "var(--text-secondary)"
          active:
            background: "var(--list-selected-bg)"
            borderColor: "var(--border-default)"
            color: "var(--text-primary)"
            fontWeight: "600"

    result-item:
      maxPerGroup: "6 items (only when query is active; no cap when query is empty — all indexed items expand by default)"
      height: "auto (content-driven, ~44px per row)"
      padding: "6px 10px"
      borderRadius: "8px"
      margin: "0 2px"
      background: "transparent"
      border: "1px solid transparent"
      hover:
        background: "var(--list-hover-bg)"
        borderColor: "var(--list-hover-ring)"
      active (selected):
        background: "var(--list-selected-bg)"
        borderColor: "var(--list-selected-ring)"

    result-item-icon:
      size: "30×30px"
      borderRadius: "8px"
      background: "var(--inset)"
      color: "var(--text-secondary)"
      border: "1px solid var(--border-subtle)"
      active:
        background: "var(--content)"
        borderColor: "var(--border-default)"
        color: "var(--text-primary)"

    result-item-content:
      title:
        fontSize: "12px (text-sm)"
        fontWeight: "500"
        color: "var(--text-primary)"
        overflow: "hidden / white-space: nowrap"
        lineHeight: "1.25"
      subtitle:
        fontSize: "11px"
        color: "var(--text-tertiary)"
        fontFamily: "var(--font-mono) — monospaced for file paths"
        marginTop: "2px"
        activeColor: "var(--text-secondary)"

    result-item-meta:
      display: "flex / gap 12px / align-items center"
      badge:
        padding: "2px 8px"
        borderRadius: "999px"
        border: "1px solid var(--border-subtle)"
        color: "var(--text-tertiary)"
        fontSize: "10px / weight 600 / uppercase / tracking 0.3px"
        background: "transparent"
        active:
          background: "var(--content)"
          borderColor: "var(--border-default)"
          color: "var(--text-secondary)"
      shortcut (↵ kbd):
        minWidth: "20px / height 20px"
        padding: "0 6px"
        borderRadius: "5px"
        fontFamily: "var(--font-mono) / fontSize 11px"
        color: "var(--text-tertiary)"
        background: "var(--inset)"
        border: "1px solid var(--border-subtle)"
        opacity: "0 → 1 on hover/active"
        active:
          background: "var(--accent-soft)"
          borderColor: "var(--accent)"
          color: "var(--accent)"

  # 操作栏 (Action Bar)
  action-bar:
    height: "32px"
    padding: "6px 16px"
    borderTop: "1px solid var(--border-subtle)"
    background: "transparent"
    status:
      fontSize: "11px (text-xs)"
      color: "var(--text-tertiary)"
      maxWidth: "280px"
      activeBg: "var(--list-selected-bg) (subtle white pill when index updates)"
    hints:
      display: "flex / gap 16px / align-items center"
      kbd:
        minWidth: "16px / height 16px"
        padding: "0 4px"
        borderRadius: "4px"
        fontFamily: "var(--font-mono) / fontSize 10px"
        color: "var(--text-tertiary)"
        background: "var(--inset)"
        border: "1px solid var(--border-subtle)"
      label:
        fontSize: "10px / weight 500 / tracking 0.04em / uppercase"
        color: "var(--text-tertiary)"

  # 空态 (Empty State)
  empty-state:
    display: "flex / flex-direction: column / align-items center / justify-content center"
    padding: "40px 16px"
    icon:
      size: "32px / strokeWidth 1.5"
      color: "var(--text-quaternary)"
      opacity: "0.4"
    text:
      fontSize: "14px (text-base)"
      fontWeight: "400"
      color: "var(--text-tertiary)"
    hint:
      fontSize: "12px (text-sm)"
      color: "var(--text-quaternary)"

  # 滚动条 (Scrollbar)
  scrollbar:
    width: "6px"
    thumb:
      background: "rgba(255, 255, 255, 0.10)"
      borderRadius: "999px"
      hover: "rgba(255, 255, 255, 0.18)"
    track: "transparent"

# ============================================================
# 配色原则 (Color Principles)
# ============================================================
color-principles:
  no-color: |
    The system uses NO chromatic colors. All visual hierarchy is built
    from grayscale tokens, hairline borders, and glass blur. The only
    chromatic color allowed is danger (#f87171) for destructive actions
    and error states.
  accent-white: |
    In dark mode, accent color is white (#ffffff). In light mode, accent
    is near-black (#18181b). Selected rows use --list-selected-bg which
    is a subtle 8% white/black overlay — not a colored fill.
  surface-ladder: |
    6-step grayscale ladder (canvas → canvas-elevated → rail → panel →
    content → elevated → inset) provides enough headroom for elevation
    without ever needing chromatic distinction.
  borders-first: |
    Borders are 1px hairline at 6%/10%/18%/28% opacity. Group separation,
    card edges, input fields, focus rings all use the same border
    vocabulary — no shadows-as-borders.

known-gaps:
  - "Hover states documented per-component but not exhaustively — Raycast-style rich hover on command-palette rows not fully captured"
  - "Light mode shadows are subtle; dark mode shadows use higher opacity for visibility — system relies more on glass blur than shadows for elevation"
  - "Music module has its own sub-token layer (--ww-music-*) that extends the core system — documented separately in music-shared.css"
  - "Cloud-abode module uses a distinct editorial/velvety visual language on top of the core tokens"
  - "Form validation states beyond the warn-tinted border are minimal"
  - "No brand gradient / hero stripe system — MonoTools uses neutral accents instead of Raycast's red gradient band"
  - "Command palette mockup component (Raycast-style hero) not yet implemented as a standalone component"
