/**
 * Model definitions for the dashboard component
 */

export interface DashboardStat {
  title: string;
  value: number;
  icon: string;
  description: string;
  color: string;
}

export interface DashboardFeatureCard {
  title: string;
  subtitle: string;
  description: string;
  icon: string;
  routerLink: string;
  buttonText: string;
  requiresAuthentication: boolean;
}

export interface DashboardActivity {
  id: number;
  title: string;
  description: string;
  time: string;
  icon: string;
}
