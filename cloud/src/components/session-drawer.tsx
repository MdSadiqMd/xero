import { Button } from "@xero/ui/components/ui/button";
import {
	Sheet,
	SheetClose,
	SheetContent,
	SheetDescription,
	SheetHeader,
	SheetTitle,
	SheetTrigger,
} from "@xero/ui/components/ui/sheet";
import { Menu, X } from "lucide-react";
import { type ReactNode, useCallback, useState } from "react";

import type { CloudSession } from "#/lib/auth/session";
import type {
	RemoteProjectSummary,
	VisibleSessionSummary,
} from "#/lib/relay/session-store";

import { SessionListPanel } from "./session-list-panel";

interface SessionDrawerProps {
	session: CloudSession;
	visibleSessions: VisibleSessionSummary[];
	projects?: RemoteProjectSummary[];
	currentSessionKey: string | null;
	open?: boolean;
	onOpenChange?: (open: boolean) => void;
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
	trigger?: ReactNode;
}

export function SessionDrawer({
	session,
	visibleSessions,
	projects = [],
	currentSessionKey,
	open,
	onOpenChange,
	onSelectSession,
	onSelectProject,
	onSetSessionRemoteVisibility,
	onArchiveSession,
	onSignOut,
	trigger,
}: SessionDrawerProps) {
	const [internalOpen, setInternalOpen] = useState(false);
	const isOpen = open ?? internalOpen;
	const setIsOpen = useCallback(
		(next: boolean) => {
			setInternalOpen(next);
			onOpenChange?.(next);
		},
		[onOpenChange],
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
				className="cloud-session-drawer-content flex w-[86vw] max-w-[340px] flex-col gap-0 border-l border-border bg-background p-0 sm:w-[340px] [&>button.absolute]:hidden"
			>
				<SheetHeader className="sr-only">
					<SheetTitle>Desktop sessions</SheetTitle>
					<SheetDescription>
						Browse desktop sessions and manage the signed-in account.
					</SheetDescription>
				</SheetHeader>
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
					onAfterSelectSession={() => setIsOpen(false)}
					onProjectPickerOpenChange={(pickerOpen) => {
						if (pickerOpen) setIsOpen(false);
					}}
					closeSlot={
						<SheetClose asChild>
							<button
								type="button"
								aria-label="Close"
								className="-mr-1 flex size-7 shrink-0 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-accent hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60"
							>
								<X className="h-4 w-4" />
							</button>
						</SheetClose>
					}
				/>
			</SheetContent>
		</Sheet>
	);
}
