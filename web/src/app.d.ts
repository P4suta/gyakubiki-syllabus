/// <reference types="vite-plugin-pwa/client" />

declare const __APP_VERSION__: string
declare const __APP_COMMIT__: string

interface Window {
	__GYAKUBIKI_E2E__?: {
		crashNextWorkerRequest(): void
		stallNextWorkerRequest(): void
	}
}
