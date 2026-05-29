export interface XeroCloudServiceWorkerRegistrationOptions {
	serviceWorker?: ServiceWorkerContainer;
	onUpdateReady?: (registration: ServiceWorkerRegistration) => void;
	updateCheckIntervalMs?: number;
}

export interface XeroCloudServiceWorkerUpdateOptions {
	serviceWorker?: ServiceWorkerContainer;
	reload?: () => void;
}

export function registerXeroCloudServiceWorker(
	options: XeroCloudServiceWorkerRegistrationOptions = {},
): () => void {
	const serviceWorker = options.serviceWorker ?? getBrowserServiceWorker();
	if (!serviceWorker) return noop;

	let disposed = false;
	let cleanupUpdateChecks: () => void = noop;
	const notifyUpdateReady = (registration: ServiceWorkerRegistration) => {
		if (!disposed) options.onUpdateReady?.(registration);
	};
	const maybeCheckForUpdate = (registration: ServiceWorkerRegistration) => {
		if (typeof registration.update !== "function") return;
		void registration.update().catch(noop);
	};

	void serviceWorker.register("/sw.js", { scope: "/" }).then((registration) => {
		if (disposed) return;
		if (registration.waiting && serviceWorker.controller) {
			notifyUpdateReady(registration);
		}
		maybeCheckForUpdate(registration);

		registration.addEventListener("updatefound", () => {
			const installing = registration.installing;
			if (!installing) return;

			installing.addEventListener("statechange", () => {
				if (
					installing.state === "installed" &&
					serviceWorker.controller &&
					registration.waiting
				) {
					notifyUpdateReady(registration);
				}
			});
		});

		if (typeof window === "undefined" || typeof document === "undefined") {
			return;
		}

		const handleFocus = () => maybeCheckForUpdate(registration);
		const handleVisibilityChange = () => {
			if (document.visibilityState === "visible") {
				maybeCheckForUpdate(registration);
			}
		};
		window.addEventListener("focus", handleFocus);
		document.addEventListener("visibilitychange", handleVisibilityChange);
		const updateInterval =
			options.updateCheckIntervalMs === 0
				? null
				: window.setInterval(
						() => maybeCheckForUpdate(registration),
						options.updateCheckIntervalMs ?? 5 * 60_000,
					);
		cleanupUpdateChecks = () => {
			window.removeEventListener("focus", handleFocus);
			document.removeEventListener("visibilitychange", handleVisibilityChange);
			if (updateInterval !== null) window.clearInterval(updateInterval);
		};
		if (disposed) cleanupUpdateChecks();
	});

	return () => {
		disposed = true;
		cleanupUpdateChecks();
	};
}

export async function unregisterXeroCloudServiceWorkers(
	serviceWorker: ServiceWorkerContainer,
	scopeOrigin = getBrowserOrigin(),
): Promise<void> {
	const registrations = await getServiceWorkerRegistrations(serviceWorker);
	const origin = parseOrigin(scopeOrigin);
	await Promise.all(
		registrations
			.filter((registration) => {
				if (!origin) return true;
				return registration.scope.startsWith(`${origin}/`);
			})
			.map((registration) => registration.unregister()),
	);
}

export function activateWaitingXeroCloudServiceWorker(
	registration: ServiceWorkerRegistration,
	options: XeroCloudServiceWorkerUpdateOptions = {},
): void {
	const serviceWorker = options.serviceWorker ?? getBrowserServiceWorker();
	const reload = options.reload ?? getBrowserReload();
	const waiting = registration.waiting;
	if (!serviceWorker || !waiting) return;

	let didReload = false;
	const handleControllerChange = () => {
		if (didReload) return;
		didReload = true;
		serviceWorker.removeEventListener(
			"controllerchange",
			handleControllerChange,
		);
		reload();
	};

	serviceWorker.addEventListener("controllerchange", handleControllerChange);
	waiting.postMessage({ type: "SKIP_WAITING" });
}

async function getServiceWorkerRegistrations(
	serviceWorker: ServiceWorkerContainer,
): Promise<readonly ServiceWorkerRegistration[]> {
	const compatibleServiceWorker = serviceWorker as ServiceWorkerContainer & {
		getRegistrations?: ServiceWorkerContainer["getRegistrations"];
		getRegistration?: ServiceWorkerContainer["getRegistration"];
	};
	if (compatibleServiceWorker.getRegistrations) {
		return compatibleServiceWorker.getRegistrations();
	}
	const registration = await compatibleServiceWorker.getRegistration?.();
	return registration ? [registration] : [];
}

function getBrowserServiceWorker(): ServiceWorkerContainer | undefined {
	if (typeof navigator === "undefined") return undefined;
	return navigator.serviceWorker;
}

function getBrowserOrigin(): string {
	if (typeof window === "undefined") return "https://cloud.xeroshell.test";
	return window.location.origin;
}

function getBrowserReload(): () => void {
	if (typeof window === "undefined") return noop;
	return () => window.location.reload();
}

function parseOrigin(value: string): string | undefined {
	try {
		return new URL(value).origin;
	} catch {
		return undefined;
	}
}

function noop() {}
