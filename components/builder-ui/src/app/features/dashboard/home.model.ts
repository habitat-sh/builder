/**
 * Model definitions for the home component
 */

/**
 * Interface for a feature card displayed on the home page
 */
export interface HomeFeatureCard {
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
 * Interface for home page user statistics summary
 */
export interface HomeStatsSummary {
  origins: number;
  packages: number;
  totalBuilds: number;
  successfulBuilds: number;
  buildSuccessRate: number;
}
