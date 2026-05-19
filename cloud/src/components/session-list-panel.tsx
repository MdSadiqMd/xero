import {
	Avatar,
	AvatarFallback,
	AvatarImage,
} from "@xero/ui/components/ui/avatar";
import { Badge } from "@xero/ui/components/ui/badge";
import { Button } from "@xero/ui/components/ui/button";
import {
	Empty,
	EmptyDescription,
	EmptyHeader,
	EmptyMedia,
	EmptyTitle,
} from "@xero/ui/components/ui/empty";
import { cn } from "@xero/ui/lib/utils";
import { ArrowUpRight, Power, Share2, X } from "lucide-react";
import { type ReactNode, useCallback, useState } from "react";

import { InstallAppAction } from "#/components/install-app-action";
import { NewSessionPicker } from "#/components/new-session-picker";
import type { CloudSession } from "#/lib/auth/session";
import type {
	RemoteProjectSummary,
	VisibleSessionSummary,
} from "#/lib/relay/session-store";

import { SessionListRow } from "./session-list-row";

interface SessionListPanelProps {
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
	onAfterSelectSession?: () => void;
	onProjectPickerOpenChange?: (open: boolean) => void;
	titleAs?: "h2" | "div";
	titleSlot?: ReactNode;
	closeSlot?: ReactNode;
	showCount?: boolean;
	headerClassName?: string;
}

export function SessionListPanel({
	session,
	visibleSessions,
	projects = [],
	currentSessionKey,
	onSelectSession,
	onSelectProject,
	onSetSessionRemoteVisibility,
	onArchiveSession,
	onSignOut,
	onAfterSelectSession,
	onProjectPickerOpenChange,
	titleAs = "div",
	titleSlot,
	closeSlot,
	showCount = true,
	headerClassName,
}: SessionListPanelProps) {
	const [pendingSessionAction, setPendingSessionAction] = useState<{
		key: string;
		action: "visibility" | "archive";
	} | null>(null);
	const total = visibleSessions.length;

	const handleSelectSession = useCallback(
		async (summary: VisibleSessionSummary) => {
			if (!summary.remoteVisible) {
				if (!onSetSessionRemoteVisibility) return;
				const key = `${summary.computerId}:${summary.sessionId}`;
				setPendingSessionAction({ key, action: "visibility" });
				try {
					const didRequest = await onSetSessionRemoteVisibility(summary, true);
					if (!didRequest) return;
				} catch {
					return;
				} finally {
					setPendingSessionAction(null);
				}
			}
			onSelectSession(summary.computerId, summary.sessionId);
			onAfterSelectSession?.();
		},
		[onAfterSelectSession, onSelectSession, onSetSessionRemoteVisibility],
	);
	const handleSetSessionRemoteVisibility = useCallback(
		async (summary: VisibleSessionSummary, visible: boolean) => {
			if (!onSetSessionRemoteVisibility) return;
			const key = `${summary.computerId}:${summary.sessionId}`;
			setPendingSessionAction({ key, action: "visibility" });
			try {
				await onSetSessionRemoteVisibility(summary, visible);
			} catch {
				// The authoritative session list will remain unchanged if the command fails.
			} finally {
				setPendingSessionAction(null);
			}
		},
		[onSetSessionRemoteVisibility],
	);
	const handleArchiveSession = useCallback(
		async (summary: VisibleSessionSummary) => {
			if (!onArchiveSession) return;
			const key = `${summary.computerId}:${summary.sessionId}`;
			setPendingSessionAction({ key, action: "archive" });
			try {
				await onArchiveSession(summary);
			} catch {
				// The desktop remains authoritative if the command fails.
			} finally {
				setPendingSessionAction(null);
			}
		},
		[onArchiveSession],
	);

	const TitleTag = titleAs;
	const titleNode = titleSlot ?? (
		<TitleTag className="truncate text-sm font-medium tracking-tight text-foreground">
			Desktop sessions
		</TitleTag>
	);

	return (
		<>
			<div
				className={cn(
					"gap-0 border-b border-border px-4 pb-3 pt-[max(env(safe-area-inset-top),0.75rem)]",
					headerClassName,
				)}
			>
				<div className="flex items-center justify-between gap-2">
					<div className="flex min-w-0 items-center gap-2">
						{titleNode}
						{showCount && total > 0 ? (
							<Badge
								variant="secondary"
								className="font-mono text-[10px] tabular-nums text-muted-foreground"
							>
								{total}
							</Badge>
						) : null}
					</div>
					<div className="flex shrink-0 items-center gap-1">
						{onSelectProject ? (
							<NewSessionPicker
								projects={projects}
								onSelectProject={(projectId) => {
									onSelectProject(projectId);
								}}
								onPickerOpenChange={onProjectPickerOpenChange}
							/>
						) : null}
						{closeSlot}
					</div>
				</div>
			</div>

			<div className="flex flex-1 flex-col overflow-y-auto overscroll-contain">
				{total === 0 ? (
					<div className="flex min-h-full w-full flex-1 items-center justify-center">
						<Empty className="border-0">
							<EmptyHeader>
								<EmptyMedia variant="icon">
									<Share2 className="size-5 text-muted-foreground" />
								</EmptyMedia>
								<EmptyTitle className="text-sm font-medium text-foreground">
									No desktop sessions yet
								</EmptyTitle>
								<EmptyDescription className="text-xs">
									Open Xero on your desktop to make sessions available here.
								</EmptyDescription>
							</EmptyHeader>
						</Empty>
					</div>
				) : (
					<div className="flex flex-col gap-3">
						<ul className="flex flex-col">
							{visibleSessions.map((summary) => {
								const key = `${summary.computerId}:${summary.sessionId}`;
								const pendingAction =
									pendingSessionAction?.key === key
										? pendingSessionAction.action
										: undefined;
								return (
									<li key={key}>
										<SessionListRow
											summary={summary}
											isActive={currentSessionKey === key}
											onSelect={() => void handleSelectSession(summary)}
											onSetRemoteVisibility={
												onSetSessionRemoteVisibility
													? (visible) =>
															handleSetSessionRemoteVisibility(summary, visible)
													: undefined
											}
											onArchive={
												onArchiveSession
													? () => void handleArchiveSession(summary)
													: undefined
											}
											isPending={pendingSessionAction?.key === key}
											pendingAction={pendingAction}
										/>
									</li>
								);
							})}
						</ul>
					</div>
				)}
			</div>

			<footer className="flex items-center gap-3 border-t border-border bg-background px-4 py-3 pb-[max(env(safe-area-inset-bottom),0.75rem)]">
				<a
					href={`https://github.com/${session.githubLogin}`}
					target="_blank"
					rel="noreferrer noopener"
					className="group flex min-w-0 flex-1 items-center gap-3 rounded-md px-1.5 py-1 -mx-1.5 -my-1 transition-colors hover:bg-accent"
				>
					<Avatar className="h-8 w-8 ring-1 ring-border">
						{session.avatarUrl ? (
							<AvatarImage src={session.avatarUrl} alt={session.githubLogin} />
						) : null}
						<AvatarFallback className="text-xs">
							{session.githubLogin.slice(0, 2).toUpperCase()}
						</AvatarFallback>
					</Avatar>
					<div className="flex min-w-0 flex-1 items-center gap-1">
						<span className="truncate text-sm font-medium text-foreground">
							{session.githubLogin}
						</span>
						<ArrowUpRight className="h-3 w-3 shrink-0 opacity-0 transition-opacity group-hover:opacity-60" />
					</div>
				</a>
				<InstallAppAction variant="compact" />
				<Button
					variant="ghost"
					size="icon"
					aria-label="Sign out"
					onClick={onSignOut}
					className="shrink-0 text-muted-foreground hover:text-destructive hover:bg-destructive/10"
				>
					<Power className="h-4 w-4" />
				</Button>
			</footer>
		</>
	);
}

export { X as SessionListPanelCloseIcon };
