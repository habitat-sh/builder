// Origin model definitions
export interface Origin {
  id?: string;
  name: string;
  default_package_visibility: string;
  package_count?: number;
  is_private?: boolean;
}

export interface OriginInvitation {
  id: string;
  origin: string;
  origin_id: string;
  account_id: string;
  account_name: string;
  owner_id: string;
  isInvite?: boolean;  // UI flag, not from API
}

export type OriginItem = Origin | OriginInvitation;
