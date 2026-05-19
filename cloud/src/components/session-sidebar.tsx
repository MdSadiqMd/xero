import { cn } from "@xero/ui/lib/utils";

import { BrandLogo } from "#/components/brand-logo";
import type { CloudSession } from "#/lib/auth/session";
import type {
	RemoteProjectSummary,
	VisibleSessionSummary,
} from "#/lib/relay/session-store";

import { SessionListPanel } from "./session-list-panel";

interface SessionSidebarProps {
	session: CloudSession;
	visibleSessions: VisibleSessionSummary[];
	projects?: RemoteProjectSummary[];
	currentSessionKey: string | null;
	onSelectSession: (computerId: string, sessionId: string) => void;
	onSelectProject?: (projectId: string) => void;
	onSetSessionRemoteVisibility?: (
		summary: VisibleSessionSummary,
		visible: boolean,
	) => boolean | Promise<boolean>;
	onArchiveSession?: (
		summary: VisibleSessionSummary,
	) => boolean | Promise<boolean>;
	onSignOut: () => void;
	className?: string;
}

export function SessionSidebar({
	session,
	visibleSessions,
	projects = [],
	currentSessionKey,
	onSelectSession,
	onSelectProject,
	onSetSessionRemoteVisibility,
	onArchiveSession,
	onSignOut,
	className,
}: SessionSidebarProps) {
	return (
		<aside
			aria-label="Desktop sessions"
			className={cn(
				"hidden h-dvh w-[300px] shrink-0 flex-col gap-0 border-r border-border bg-card/40 lg:flex",
				className,
			)}
		>
			<SessionListPanel
				session={session}
				visibleSessions={visibleSessions}
				projects={projects}
				currentSessionKey={currentSessionKey}
				onSelectSession={onSelectSession}
				onSelectProject={onSelectProject}
				onSetSessionRemoteVisibility={onSetSessionRemoteVisibility}
				onArchiveSession={onArchiveSession}
				onSignOut={onSignOut}
				showCount={false}
				titleSlot={
					<a
						href="/sessions"
						className="flex min-w-0 items-center gap-2 rounded-md px-1 py-1 -mx-1 transition-colors hover:bg-accent/40"
						aria-label="Xero"
					>
						<BrandLogo className="size-5 shrink-0" aria-hidden />
						<span className="truncate text-sm font-semibold tracking-tight text-foreground">
							Xero
						</span>
					</a>
				}
			/>
		</aside>
	);
}
