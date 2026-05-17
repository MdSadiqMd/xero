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
import {
	Sheet,
	SheetClose,
	SheetContent,
	SheetDescription,
	SheetHeader,
	SheetTitle,
	SheetTrigger,
} from "@xero/ui/components/ui/sheet";
import { ArrowUpRight, Menu, Power, Share2, X } from "lucide-react";
import { type ReactNode, useCallback, useMemo, useState } from "react";

import type { CloudSession } from "#/lib/auth/session";
import type { VisibleSessionSummary } from "#/lib/relay/session-store";

import { SessionListRow } from "./session-list-row";

interface SessionDrawerProps {
	session: CloudSession;
	visibleSessions: VisibleSessionSummary[];
	currentSessionKey: string | null;
	onSelectSession: (computerId: string, sessionId: string) => void;
	onSignOut: () => void;
	trigger?: ReactNode;
}

interface ComputerGroup {
	computerId: string;
	computerName: string;
	sessions: VisibleSessionSummary[];
}

function groupByComputer(sessions: VisibleSessionSummary[]): ComputerGroup[] {
	const groups = new Map<string, ComputerGroup>();
	for (const s of sessions) {
		const existing = groups.get(s.computerId);
		if (existing) {
			existing.sessions.push(s);
		} else {
			groups.set(s.computerId, {
				computerId: s.computerId,
				computerName: s.computerName ?? "Desktop",
				sessions: [s],
			});
		}
	}
	return Array.from(groups.values());
}

export function SessionDrawer({
	session,
	visibleSessions,
	currentSessionKey,
	onSelectSession,
	onSignOut,
	trigger,
}: SessionDrawerProps) {
	const [isOpen, setIsOpen] = useState(false);
	const groups = useMemo(
		() => groupByComputer(visibleSessions),
		[visibleSessions],
	);
	const total = visibleSessions.length;
	const hasMultipleComputers = groups.length > 1;

	const handleSelectSession = useCallback(
		(computerId: string, sessionId: string) => {
			setIsOpen(false);
			onSelectSession(computerId, sessionId);
		},
		[onSelectSession],
	);

	return (
		<Sheet open={isOpen} onOpenChange={setIsOpen}>
			<SheetTrigger asChild>
				{trigger ?? (
					<Button variant="ghost" size="icon" aria-label="Open sessions list">
						<Menu className="h-5 w-5" />
					</Button>
				)}
			</SheetTrigger>
			<SheetContent
				side="right"
				onOpenAutoFocus={(event) => event.preventDefault()}
				className="flex w-[86vw] max-w-[340px] flex-col gap-0 border-l border-border bg-background p-0 sm:w-[340px] [&>button.absolute]:hidden"
			>
				<SheetHeader className="gap-0 border-b border-border px-4 py-3">
					<div className="flex items-center justify-between gap-2">
						<div className="flex min-w-0 items-center gap-2">
							<SheetTitle className="truncate text-sm font-medium tracking-tight text-foreground">
								Shared sessions
							</SheetTitle>
							{total > 0 ? (
								<Badge
									variant="secondary"
									className="font-mono text-[10px] tabular-nums text-muted-foreground"
								>
									{total}
								</Badge>
							) : null}
						</div>
						<SheetClose asChild>
							<button
								type="button"
								aria-label="Close"
								className="-mr-1 flex size-7 shrink-0 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-accent hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60"
							>
								<X className="h-4 w-4" />
							</button>
						</SheetClose>
					</div>
					<SheetDescription className="sr-only">
						Browse sessions shared to the web and manage the signed-in account.
					</SheetDescription>
				</SheetHeader>

				<div className="flex flex-1 flex-col overflow-y-auto overscroll-contain">
					{total === 0 ? (
						<div className="flex min-h-full w-full flex-1 items-center justify-center">
							<Empty className="border-0">
								<EmptyHeader>
									<EmptyMedia variant="icon">
										<Share2 className="size-5 text-muted-foreground" />
									</EmptyMedia>
									<EmptyTitle className="text-sm font-medium text-foreground">
										Nothing shared yet
									</EmptyTitle>
									<EmptyDescription className="text-xs">
										Open Xero on your desktop, find a session, and enable{" "}
										<span className="font-medium text-foreground">
											Share to web
										</span>
										. It&apos;ll appear here instantly.
									</EmptyDescription>
								</EmptyHeader>
							</Empty>
						</div>
					) : (
						<div className="flex flex-col gap-4 px-2 py-3">
							{groups.map((group) => (
								<section key={group.computerId} className="flex flex-col gap-1">
									{hasMultipleComputers ? (
										<div className="flex items-center gap-2 px-3 pt-1 pb-1.5">
											<span className="truncate font-mono text-[10px] uppercase tracking-[0.18em] text-muted-foreground">
												{group.computerName}
											</span>
											<span aria-hidden className="h-px flex-1 bg-border/60" />
											<span className="font-mono text-[10px] tabular-nums text-muted-foreground/70">
												{group.sessions.length}
											</span>
										</div>
									) : null}
									<ul className="flex flex-col gap-0.5">
										{group.sessions.map((summary) => {
											const key = `${summary.computerId}:${summary.sessionId}`;
											return (
												<li key={key}>
													<SessionListRow
														summary={summary}
														isActive={currentSessionKey === key}
														showComputer={!hasMultipleComputers}
														onSelect={() =>
															handleSelectSession(
																summary.computerId,
																summary.sessionId,
															)
														}
													/>
												</li>
											);
										})}
									</ul>
								</section>
							))}
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
								<AvatarImage
									src={session.avatarUrl}
									alt={session.githubLogin}
								/>
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
			</SheetContent>
		</Sheet>
	);
}
