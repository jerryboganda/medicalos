// The one list of destinations (design.md § App shell). The desktop rail,
// the mobile sheet, the bottom tab bar and the ⌘K palette all read it —
// add a destination here, once. `testid`s are e2e contracts: keep them.

import type { IconName } from './components/icons';

export type NavGroup = 'Study' | 'Learn' | 'You' | 'Workspaces';

export type NavItem = {
	href: string;
	label: string;
	icon: IconName;
	group: NavGroup;
	testid: string;
	/** Shown in the mobile tab bar (§6's five primary destinations). */
	tab?: boolean;
	/** Extra words the palette matches on. */
	keywords?: string;
};

export const NAV: NavItem[] = [
	{ href: '/today', label: 'Today', icon: 'today', group: 'Study', testid: 'nav-today', tab: true, keywords: 'plan home goal' },
	{ href: '/practice', label: 'Practice', icon: 'practice', group: 'Study', testid: 'nav-practice', tab: true, keywords: 'qbank mock competition league session' },
	{ href: '/review', label: 'Review', icon: 'review', group: 'Study', testid: 'nav-review', keywords: 'flashcards cards spaced repetition' },
	{ href: '/scenarios', label: 'Simulations', icon: 'scenarios', group: 'Study', testid: 'nav-scenarios', keywords: 'osce scenario station' },
	{ href: '/library', label: 'Library', icon: 'library', group: 'Learn', testid: 'nav-library', tab: true, keywords: 'articles reading imports' },
	{ href: '/imaging', label: 'Images', icon: 'imaging', group: 'Learn', testid: 'nav-imaging', keywords: 'radiology anatomy image studies' },
	{ href: '/notes', label: 'Notes', icon: 'notes', group: 'Learn', testid: 'nav-notes', keywords: 'notebook' },
	{ href: '/offline', label: 'Offline packs', icon: 'offline', group: 'Learn', testid: 'nav-offline', keywords: 'downloads packs' },
	{ href: '/coach', label: 'Coach', icon: 'coach', group: 'You', testid: 'nav-coach', tab: true, keywords: 'ai tutor explain' },
	{ href: '/progress', label: 'Progress', icon: 'progress', group: 'You', testid: 'nav-progress', tab: true, keywords: 'mastery evidence heatmap timeline' },
	{ href: '/notifications', label: 'Notifications', icon: 'bell', group: 'You', testid: 'nav-notifications', keywords: 'inbox preferences' },
	{ href: '/community', label: 'Community', icon: 'community', group: 'You', testid: 'nav-community', keywords: 'groups duels profile' },
	{ href: '/faculty', label: 'Faculty', icon: 'faculty', group: 'Workspaces', testid: 'nav-faculty', keywords: 'institution cohorts assignments' },
	{ href: '/admin', label: 'Console', icon: 'console', group: 'Workspaces', testid: 'nav-admin', keywords: 'editorial admin import settings' }
];

export const NAV_GROUPS: NavGroup[] = ['Study', 'Learn', 'You', 'Workspaces'];

/** Deep pages the palette can open but the rail doesn't list. */
export const PALETTE_EXTRAS: Pick<NavItem, 'href' | 'label' | 'icon' | 'group' | 'keywords'>[] = [
	{ href: '/admin/dashboard', label: 'Owner dashboard', icon: 'dashboard', group: 'Workspaces', keywords: 'metrics admin' },
	{ href: '/admin/articles', label: 'Article workspace', icon: 'article', group: 'Workspaces', keywords: 'authoring library admin' },
	{ href: '/admin/image-annotations', label: 'Image annotation review', icon: 'imaging', group: 'Workspaces', keywords: 'annotations admin' },
	{ href: '/scenarios/team', label: 'Simulation team', icon: 'community', group: 'Study', keywords: 'handover scenario team' }
];

/** `/library/articles/x` keeps Library lit; `/admin/dashboard` keeps Console lit. */
export function isActive(path: string, href: string): boolean {
	return path === href || path.startsWith(`${href}/`);
}
