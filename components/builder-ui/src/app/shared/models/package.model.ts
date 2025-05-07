import { Injectable } from '@angular/core';

/**
 * Package identifier model representing a unique package in Habitat Builder
 */
export interface PackageIdent {
  origin: string;
  name: string;
  version?: string;
  release?: string;
}

/**
 * Package visibility enum - controls who can access the package
 */
export enum PackageVisibility {
  Public = 'public',
  Private = 'private',
  Hidden = 'hidden'
}

/**
 * Package type enum - standard or native
 */
export enum PackageType {
  Standard = 'standard',
  Native = 'native'
}

/**
 * Represents a target platform for packages
 */
export interface PackageTarget {
  name: string;
  platform: string;
  architecture: string;
}

/**
 * Represents a complete package object with all details
 */
export interface Package {
  id: string;
  ident: PackageIdent;
  visibility: PackageVisibility;
  type: PackageType;
  target: PackageTarget;
  checksum: string;
  manifest: string;
  config: string;
  deps: PackageIdent[];
  tdeps: PackageIdent[];
  buildDeps: PackageIdent[];
  buildTdeps: PackageIdent[];
  exposes: number[];
  channels: string[];
  platforms: string[];
  createdAt: Date;
  updatedAt: Date;
  origin: string;
  ownerId: string;
}

/**
 * Simplified package data for listing in tables/lists
 */
export interface PackageSummary {
  ident: PackageIdent;
  target: PackageTarget;
  visibility: PackageVisibility;
  type: PackageType;
  channels: string[];
  updatedAt: Date;
}

/**
 * Information about the latest stable package version
 */
export interface LatestPackage {
  ident: PackageIdent;
  target: PackageTarget;
  releaseCount: number;
  updatedAt: Date;
  platforms: string[];
}

/**
 * Package search parameters 
 */
export interface PackageSearch {
  origin?: string;
  name?: string;
  version?: string;
  release?: string;
  target?: string;
  visibility?: PackageVisibility[];
  page?: number;
  limit?: number;
  query?: string;
}

/**
 * Package search results
 */
export interface PackageSearchResult {
  packages: PackageSummary[];
  totalCount: number;
  page: number;
  perPage: number;
}
