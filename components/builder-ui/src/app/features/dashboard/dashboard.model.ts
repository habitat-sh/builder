/**
 * Model definitions for the dashboard component
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
