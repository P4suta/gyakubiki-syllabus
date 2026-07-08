<script lang="ts">
import type { Snippet } from 'svelte'
import { matchesDesktop, useDesktop } from '../lib/breakpoint.svelte'
import { haptic, rubberBand, shouldCommit } from '../lib/gestures'

interface Props {
	/** Perform the actual teardown (the parent unmounts this via `{#if}`). */
	onclose: () => void
	/** Accessible name for the dialog. */
	ariaLabel: string
	/** Non-scrolling top region (also a drag handle). Receives a `close` fn. */
	header?: Snippet<[close: () => void]>
	/** Non-scrolling bottom region (safe-area padded). Receives a `close` fn. */
	footer?: Snippet<[close: () => void]>
	/** Scrollable body. */
	children: Snippet
}

let { onclose, ariaLabel, header, footer, children }: Props = $props()

const initialMobile = !matchesDesktop()

const desktop = useDesktop()
const isDesktop = $derived(desktop.current)
let dialogEl = $state<HTMLDialogElement>()
let sheetEl = $state<HTMLElement>()
let bodyEl = $state<HTMLElement>()

// Render in the top layer: `showModal()` lifts the dialog above every stacking
// context (no z-index) and traps focus. Native Esc fires `cancel`; we route it
// through our own dismiss so the history/back close-path stays single. (jsdom
// lacks showModal — fall back to the `open` attribute so component tests run.)
$effect(() => {
	const el = dialogEl
	if (!el) return
	try {
		el.showModal()
	} catch {
		el.open = true
	}
	return () => {
		try {
			el.close()
		} catch {
			el.open = false
		}
	}
})

function onCancel(e: Event) {
	e.preventDefault() // keep close on our single path, not the native one
	dismiss()
}

// Sheet offset from its resting position. Starts a screen below on mobile so the
// first paint is off-screen and the open effect can slide it up.
let dragY = $state(initialMobile ? 9999 : 0)
let settling = $state(initialMobile)
let sheetHeight = 400

// Backdrop dims as the sheet is dragged away, reaching ~0 when fully off-screen.
const backdropOpacity = $derived(
	dragY > 0 && sheetHeight ? Math.max(0, 1 - dragY / sheetHeight) : 1,
)

// --- Close routing: every dismissal funnels through the history entry so the
// device Back button and in-app gestures share one path and leave no dangling
// history state (see requestClose / the popstate effect). ---
let pushed = false
let consumed = false
let closing = false

function actualClose() {
	if (closing) return
	closing = true
	onclose()
}

function requestClose() {
	if (closing) return
	if (pushed && !consumed && typeof history !== 'undefined') {
		history.back() // → popstate → actualClose
	} else {
		actualClose()
	}
}

/** User-initiated close (×, backdrop, Esc, footer, swipe): slide out on mobile. */
function dismiss() {
	if (isDesktop) {
		requestClose()
		return
	}
	settling = true
	sheetHeight = sheetEl?.offsetHeight || window.innerHeight
	dragY = sheetHeight
	window.setTimeout(requestClose, 240)
}

// Slide up once, on mount (mobile only).
$effect(() => {
	if (!initialMobile) return
	const raf = requestAnimationFrame(() => {
		sheetHeight = sheetEl?.offsetHeight || window.innerHeight
		dragY = 0
	})
	const t = window.setTimeout(() => {
		settling = false
	}, 300)
	return () => {
		cancelAnimationFrame(raf)
		clearTimeout(t)
	}
})

// Push a history entry so the device/browser Back button closes the sheet
// instead of leaving the page. A single popstate is the only caller of
// actualClose; on unmount by any other path we pop our lingering entry.
$effect(() => {
	if (typeof history === 'undefined') return
	history.pushState({ __sheet: true }, '')
	pushed = true
	const onPop = () => {
		consumed = true
		actualClose()
	}
	window.addEventListener('popstate', onPop)
	return () => {
		window.removeEventListener('popstate', onPop)
		if (pushed && !consumed) history.back()
	}
})

// --- Drag-to-dismiss (mobile). Attached natively so touchmove can be
// non-passive and preventDefault while we own the gesture. ---
let startY = 0
let startT = 0
let dragging = false
let active = false

function onStart(e: TouchEvent) {
	if (isDesktop) return
	const target = e.target as Node | null
	const inBody = !!(target && bodyEl?.contains(target))
	startY = e.touches[0].clientY
	startT = e.timeStamp
	sheetHeight = sheetEl?.offsetHeight || window.innerHeight
	dragging = true
	// Handle/header/footer drag immediately; the scroll body only when at its top.
	active = !inBody
	settling = false
}

function onMove(e: TouchEvent) {
	if (!dragging || isDesktop) return
	const dy = e.touches[0].clientY - startY
	if (!active) {
		const atTop = (bodyEl?.scrollTop ?? 0) <= 0
		if (dy > 4 && atTop) active = true
		else if (Math.abs(dy) > 4) {
			dragging = false // it's a content scroll, not a dismiss
			return
		} else return
	}
	e.preventDefault()
	dragY = dy > 0 ? dy : -rubberBand(-dy, sheetHeight)
}

function onEnd(e: TouchEvent) {
	if (!dragging || isDesktop) {
		dragging = false
		return
	}
	dragging = false
	if (!active) return
	active = false
	const touch = e.changedTouches[0]
	const dy = (touch ? touch.clientY : startY) - startY
	const speed = dy / Math.max(1, e.timeStamp - startT)
	if (dy > 0 && shouldCommit(dy, speed, sheetHeight, { velocityThreshold: 0.5 })) {
		haptic('light')
		dismiss()
	} else {
		settling = true
		dragY = 0
		window.setTimeout(() => {
			settling = false
		}, 240)
	}
}

$effect(() => {
	const el = sheetEl
	if (!el || isDesktop) return
	el.addEventListener('touchstart', onStart, { passive: true })
	el.addEventListener('touchmove', onMove, { passive: false })
	el.addEventListener('touchend', onEnd, { passive: true })
	el.addEventListener('touchcancel', onEnd, { passive: true })
	return () => {
		el.removeEventListener('touchstart', onStart)
		el.removeEventListener('touchmove', onMove)
		el.removeEventListener('touchend', onEnd)
		el.removeEventListener('touchcancel', onEnd)
	}
})
</script>

<!-- The native <dialog> exists only to put us in the top layer (showModal); the
     inner element carries the dialog semantics, so `role="presentation"` here
     keeps a single dialog in the a11y tree and lets tests target the sheet. -->
<!-- svelte-ignore a11y_no_interactive_element_to_noninteractive_role -->
<dialog bind:this={dialogEl} class="overlay" role="presentation" oncancel={onCancel}>
	<div class="fixed inset-0 flex items-end justify-center sm:items-center sm:p-5">
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="absolute inset-0 bg-overlay-backdrop backdrop-blur-md animate-fade-in"
			style="opacity: {backdropOpacity};"
			aria-hidden="true"
			onclick={dismiss}
		></div>

		<div
			bind:this={sheetEl}
			role="dialog"
			aria-modal="true"
			aria-label={ariaLabel}
			class="relative flex flex-col w-full max-h-overlay overflow-hidden bg-surface-primary rounded-t-2xl shadow-modal safe-bottom sm:max-w-lg sm:max-h-overlay-sm sm:rounded-2xl {isDesktop ? 'animate-dialog-in' : ''}"
			style="translate: 0 {isDesktop ? 0 : dragY}px; transition: {settling ? 'translate 0.26s var(--ease-spring)' : 'none'};"
		>
			<div class="flex justify-center pt-2 shrink-0 sm:hidden touch-none">
				<div class="w-9 h-1 rounded-full bg-overlay-strong"></div>
			</div>
			{#if header}
				<div class="shrink-0 touch-none">{@render header(dismiss)}</div>
			{/if}
			<div bind:this={bodyEl} class="grow overflow-auto overscroll-contain touch-pan-y">
				{@render children()}
			</div>
			{#if footer}
				<div class="shrink-0">{@render footer(dismiss)}</div>
			{/if}
		</div>
	</div>
</dialog>
