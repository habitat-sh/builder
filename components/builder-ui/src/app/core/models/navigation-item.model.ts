/**
 * Interface for navigation items used in sidebar navigation
 */
export interface NavigationItem {
  label: string;
  icon?: string;
  route?: string;
  children?: NavigationItem[];
  expanded?: boolean;
  divider?: boolean;
  permissions?: string[];
  isExternal?: boolean; // Explicit flag for external links
  exactMatch?: boolean; // For precise route matching with routerLinkActive
}
