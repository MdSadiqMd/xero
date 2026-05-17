import {
	createFileRoute,
	Outlet,
	redirect,
	useNavigate,
	useRouterState,
} from "@tanstack/react-router";
import { Button } from "@xero/ui/components/ui/button";
import {
	Empty,
	EmptyContent,
	EmptyDescription,
	EmptyHeader,
	EmptyMedia,
	EmptyTitle,
} from "@xero/ui/components/ui/empty";
import { Menu } from "lucide-react";
import { useEffect, useRef } from "react";

import { BrandLogo } from "#/components/brand-logo";
import { SessionDrawer } from "#/components/session-drawer";
import {
	type CloudSession,
	getCurrentSession,
	signOut,
} from "#/lib/auth/session";
import { sessionKey } from "#/lib/relay/session-store";
import { useAccountVisibleSessions } from "#/lib/relay/use-session-stream";

export const Route = createFileRoute("/sessions")({
	beforeLoad: async () => {
		const session = await getCurrentSession();
		if (!session) throw redirect({ to: "/" });
		return { session };
	},
	component: SessionsIndex,
});

function SessionsIndex() {
	const isSessionsIndex = useRouterState({
		select: (state) => {
			const pathname = state.location.pathname;
			return pathname === "/sessions" || pathname === "/sessions/";
		},
	});
	if (!isSessionsIndex) return <Outlet />;
	return <SessionsEmptyState />;
}

function SessionsEmptyState() {
	const { session } = Route.useRouteContext();
	const navigate = useNavigate();
	const redirectedSessionKey = useRef<string | null>(null);
	const visibleSessions = useAccountVisibleSessions(
		session.relayToken,
		session.accountId,
		session.devices,
		session.deviceId,
	);

	useEffect(() => {
		if (visibleSessions.length === 0) return;
		const first = visibleSessions[0];
		const nextSessionKey = sessionKey(first.computerId, first.sessionId);
		if (redirectedSessionKey.current === nextSessionKey) return;
		redirectedSessionKey.current = nextSessionKey;
		void navigate({
			to: "/sessions/$computerId/$sessionId",
			params: { computerId: first.computerId, sessionId: first.sessionId },
			replace: true,
		});
	}, [navigate, visibleSessions]);

	const handleSignOut = () => {
		void signOut().then(() => {
			if (typeof window !== "undefined") window.location.href = "/";
		});
	};

	return (
		<main className="flex min-h-dvh flex-col bg-background text-foreground">
			<header className="sticky top-0 z-20 flex items-center justify-between gap-2 bg-background px-4 py-3">
				<div className="flex items-center gap-2">
					<BrandLogo className="size-5" aria-label="Xero" />
					<span className="text-sm font-medium tracking-tight text-foreground">
						Xero
					</span>
				</div>
				<SessionDrawer
					session={session as CloudSession}
					visibleSessions={visibleSessions}
					currentSessionKey={null}
					onSelectSession={(computerId, sessionId) => {
						void navigate({
							to: "/sessions/$computerId/$sessionId",
							params: { computerId, sessionId },
						});
					}}
					onSignOut={handleSignOut}
					trigger={
						<Button
							type="button"
							variant="ghost"
							size="icon"
							aria-label="Open sessions list"
							className="text-muted-foreground hover:text-foreground"
						>
							<Menu className="h-4 w-4" />
						</Button>
					}
				/>
			</header>

			<div className="flex min-h-full w-full flex-1 items-center justify-center">
				<Empty className="border-0">
					<EmptyHeader>
						<EmptyMedia>
							<BrandLogo className="size-10" aria-label="Xero" />
						</EmptyMedia>
						<EmptyTitle className="text-sm font-medium text-foreground">
							No sessions are shared yet
						</EmptyTitle>
						<EmptyDescription className="text-xs">
							Open the Xero desktop app and toggle{" "}
							<span className="font-medium text-foreground">Share to web</span>{" "}
							on a session row to drive it from here.
						</EmptyDescription>
					</EmptyHeader>
					<EmptyContent>
						<SessionDrawer
							session={session as CloudSession}
							visibleSessions={visibleSessions}
							currentSessionKey={null}
							onSelectSession={(computerId, sessionId) => {
								void navigate({
									to: "/sessions/$computerId/$sessionId",
									params: { computerId, sessionId },
								});
							}}
							onSignOut={handleSignOut}
							trigger={
								<Button
									type="button"
									size="sm"
									variant="secondary"
									className="h-9 gap-2 px-4 text-[12px] font-medium"
								>
									<Menu className="h-3.5 w-3.5" />
									Open menu
								</Button>
							}
						/>
					</EmptyContent>
				</Empty>
			</div>
		</main>
	);
}
