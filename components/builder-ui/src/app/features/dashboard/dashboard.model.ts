/**
 * Model definitions for the dashboard component
 */

/**
 * Interface for a feature card displayed on the dashboard
 */
export interface DashboardFeatureCard {
  title: string;
  subtitle: string;
  description: string;
  icon: string;
  routerLink: string;
  buttonText: string;
  requiresAuthentication: boolean;
  hidden?: boolean;
  featureFlag?: string;
}

/**
 * Interface for dashboard user statistics summary
 */
export interface DashboardStatsSummary {
  origins: number;
  packages: number;
  totalBuilds: number;
  successfulBuilds: number;
  buildSuccessRate: number;
}
