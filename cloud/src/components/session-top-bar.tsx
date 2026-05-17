import { Button } from "@xero/ui/components/ui/button";
import { Menu, Plus } from "lucide-react";
import type { ReactNode } from "react";

import { BrandLogo } from "#/components/brand-logo";

interface SessionTopBarProps {
	title: string;
	onNewSession?: () => void;
	drawerTrigger?: ReactNode;
}

export function SessionTopBar({
	title,
	onNewSession,
	drawerTrigger,
}: SessionTopBarProps) {
	return (
		<header className="sticky top-0 z-20 flex items-center justify-between gap-3 bg-background px-4 py-3">
			<div className="flex min-w-0 items-center gap-2">
				<BrandLogo className="size-5 shrink-0" aria-label="Xero" />
				<span
					className="truncate text-sm font-medium text-foreground"
					title={title}
				>
					{title}
				</span>
			</div>
			<div className="flex shrink-0 items-center gap-1">
				{onNewSession ? (
					<Button
						type="button"
						variant="ghost"
						size="icon"
						aria-label="Start new session"
						onClick={onNewSession}
						className="text-muted-foreground hover:text-foreground"
					>
						<Plus className="h-4 w-4" />
					</Button>
				) : null}
				{drawerTrigger ?? (
					<Button
						type="button"
						variant="ghost"
						size="icon"
						aria-label="Open sessions list"
						className="text-muted-foreground hover:text-foreground"
					>
						<Menu className="h-4 w-4" />
					</Button>
				)}
			</div>
		</header>
	);
}
