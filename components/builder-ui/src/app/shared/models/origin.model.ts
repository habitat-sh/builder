/**
 * Origin model representing a top-level namespace for packages
 */
export interface Origin {
  id: string;
  name: string;
  ownerName: string;
  ownerId: string;
  default_package_visibility: string;
  createdAt: Date;
  updatedAt: Date;
  packageCount: number;
}

/**
 * Origin with additional statistics information
 */
export interface OriginWithStats extends Origin {
  packageCount: number;
  privatePackageCount: number;
  memberCount: number;
}

/**
 * Origin member type
 */
export enum OriginMemberRole {
  Owner = 'owner',
  Administrator = 'administrator',
  Maintainer = 'maintainer',
  Member = 'member'
}

/**
 * Origin member information
 */
export interface OriginMember {
  id: string;
  originId: string;
  originName: string;
  userId: string;
  userName: string;
  role: OriginMemberRole;
  createdAt: Date;
}

/**
 * Origin invitation
 */
export interface OriginInvitation {
  id: string;
  originId: string;
  originName: string;
  accountId: string;
  accountName: string;
  inviterId: string;
  inviterName: string;
  role: OriginMemberRole;
  createdAt: Date;
  token: string;
}

/**
 * Origin secret key
 */
export interface OriginSecretKey {
  id: string;
  name: string;
  originId: string;
  originName: string;
  revision: string;
  body: string;
  createdAt: Date;
}

/**
 * Origin public key
 */
export interface OriginPublicKey {
  id: string;
  name: string;
  originId: string;
  originName: string;
  revision: string;
  body: string;
  createdAt: Date;
}

/**
 * Origin keys
 */
export interface OriginKey {
  name: string;
  revision: string;
  originName: string;
  location: string;
  createdAt: Date;
}

/**
 * Origin integration (like GitHub)
 */
export interface OriginIntegration {
  id: string;
  originId: string;
  originName: string;
  integrationType: string;
  integration: string;
  name: string;
  body: any;
  createdAt: Date;
  updatedAt: Date;
}

/**
 * Origin search parameters
 */
export interface OriginSearch {
  query?: string;
  page?: number;
  limit?: number;
}
