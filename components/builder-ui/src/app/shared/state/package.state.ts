import { Injectable, Signal, computed, signal } from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { EMPTY, catchError, tap } from 'rxjs';
import { NotificationService } from '../../core/services/notification.service';
import { LoadingService } from '../../core/services/loading.service';
import { PackageService } from '../services/package.service';
import { 
  LatestPackage, 
  Package, 
  PackageIdent, 
  PackageSearch,
  PackageSearchResult, 
  PackageSummary 
} from '../models/package.model';

@Injectable({
  providedIn: 'root'
})
export class PackageState {
  // State signals
  private readonly _currentPackage = signal<Package | null>(null);
  private readonly _packageSearchResults = signal<PackageSearchResult | null>(null);
  private readonly _latestPackage = signal<Package | null>(null);
  private readonly _packageVersions = signal<string[]>([]);
  private readonly _packageChannels = signal<string[]>([]);
  private readonly _error = signal<string | null>(null);
  private readonly _isLoading = signal<boolean>(false);

  // Public readable signals
  public readonly currentPackage = this._currentPackage.asReadonly();
  public readonly packageSearchResults = this._packageSearchResults.asReadonly();
  public readonly latestPackage = this._latestPackage.asReadonly();
  public readonly packageVersions = this._packageVersions.asReadonly();
  public readonly packageChannels = this._packageChannels.asReadonly();
  public readonly error = this._error.asReadonly();
  public readonly isLoading = this._isLoading.asReadonly();

  // Computed signals
  public readonly totalPackages: Signal<number> = computed(() => 
    this._packageSearchResults() ? this._packageSearchResults()!.totalCount : 0);
  
  public readonly hasSearchResults: Signal<boolean> = computed(() => 
    !!this._packageSearchResults() && this._packageSearchResults()!.packages.length > 0);

  constructor(
    private packageService: PackageService,
    private loadingService: LoadingService,
    private notificationService: NotificationService
  ) {}

  /**
   * Loads a package by its identifier
   */
  loadPackage(ident: PackageIdent, target?: string): void {
    this._isLoading.set(true);
    this._error.set(null);

    this.packageService.getPackage(ident, target)
      .pipe(
        tap(() => this.loadingService.start()),
        catchError(err => {
          this._error.set(err.message || 'Failed to load package');
          this.notificationService.error('Failed to load package', err.message);
          this._isLoading.set(false);
          this.loadingService.stop();
          return EMPTY;
        }),
        tap(() => this.loadingService.stop())
      )
      .subscribe((pkg: Package) => {
        this._currentPackage.set(pkg);
        this._isLoading.set(false);
      });
  }

  /**
   * Loads the latest package version
   */
  loadLatestPackage(origin: string, name: string, target?: string, channel: string = 'stable'): void {
    this._isLoading.set(true);
    this._error.set(null);

    this.packageService.getLatestPackage(origin, name, target, channel)
      .pipe(
        tap(() => this.loadingService.start()),
        catchError(err => {
          this._error.set(err.message || 'Failed to load latest package');
          this.notificationService.error('Failed to load latest package', err.message);
          this._isLoading.set(false);
          this.loadingService.stop();
          return EMPTY;
        }),
        tap(() => this.loadingService.stop())
      )
      .subscribe((pkg: Package) => {
        this._latestPackage.set(pkg);
        this._isLoading.set(false);
      });
  }

  /**
   * Searches for packages
   */
  searchPackages(search: PackageSearch): void {
    this._isLoading.set(true);
    this._error.set(null);

    this.packageService.searchPackages(search)
      .pipe(
        tap(() => this.loadingService.start()),
        catchError(err => {
          this._error.set(err.message || 'Failed to search packages');
          this.notificationService.error('Failed to search packages', err.message);
          this._isLoading.set(false);
          this.loadingService.stop();
          return EMPTY;
        }),
        tap(() => this.loadingService.stop())
      )
      .subscribe((results: PackageSearchResult) => {
        this._packageSearchResults.set(results);
        this._isLoading.set(false);
      });
  }

  /**
   * Lists package versions
   */
  loadPackageVersions(origin: string, name: string): void {
    this._isLoading.set(true);
    this._error.set(null);

    this.packageService.listVersions(origin, name)
      .pipe(
        tap(() => this.loadingService.start()),
        catchError(err => {
          this._error.set(err.message || 'Failed to load package versions');
          this.notificationService.error('Failed to load package versions', err.message);
          this._isLoading.set(false);
          this.loadingService.stop();
          return EMPTY;
        }),
        tap(() => this.loadingService.stop())
      )
      .subscribe((versions: string[]) => {
        this._packageVersions.set(versions);
        this._isLoading.set(false);
      });
  }

  /**
   * Lists package channels
   */
  loadPackageChannels(ident: PackageIdent): void {
    this._isLoading.set(true);
    this._error.set(null);

    this.packageService.getPackageChannels(ident)
      .pipe(
        tap(() => this.loadingService.start()),
        catchError(err => {
          this._error.set(err.message || 'Failed to load package channels');
          this.notificationService.error('Failed to load package channels', err.message);
          this._isLoading.set(false);
          this.loadingService.stop();
          return EMPTY;
        }),
        tap(() => this.loadingService.stop())
      )
      .subscribe((channels: string[]) => {
        this._packageChannels.set(channels);
        this._isLoading.set(false);
      });
  }

  /**
   * Promotes a package to a channel
   */
  promoteToChannel(ident: PackageIdent, channel: string): void {
    this._isLoading.set(true);
    this._error.set(null);

    this.packageService.promoteToChannel(ident, channel)
      .pipe(
        tap(() => this.loadingService.start()),
        catchError(err => {
          this._error.set(err.message || 'Failed to promote package');
          this.notificationService.error('Failed to promote package', err.message);
          this._isLoading.set(false);
          this.loadingService.stop();
          return EMPTY;
        }),
        tap(() => this.loadingService.stop())
      )
      .subscribe(() => {
        this.notificationService.success(`Package promoted to ${channel} channel successfully`);
        this._isLoading.set(false);
        
        // Refresh channels
        this.loadPackageChannels(ident);
      });
  }

  /**
   * Demotes a package from a channel
   */
  demoteFromChannel(ident: PackageIdent, channel: string): void {
    this._isLoading.set(true);
    this._error.set(null);

    this.packageService.demoteFromChannel(ident, channel)
      .pipe(
        tap(() => this.loadingService.start()),
        catchError(err => {
          this._error.set(err.message || 'Failed to demote package');
          this.notificationService.error('Failed to demote package', err.message);
          this._isLoading.set(false);
          this.loadingService.stop();
          return EMPTY;
        }),
        tap(() => this.loadingService.stop())
      )
      .subscribe(() => {
        this.notificationService.success(`Package demoted from ${channel} channel successfully`);
        this._isLoading.set(false);
        
        // Refresh channels
        this.loadPackageChannels(ident);
      });
  }

  /**
   * Updates package visibility
   */
  updateVisibility(ident: PackageIdent, visibility: string): void {
    this._isLoading.set(true);
    this._error.set(null);

    this.packageService.updateVisibility(ident, visibility)
      .pipe(
        tap(() => this.loadingService.start()),
        catchError(err => {
          this._error.set(err.message || 'Failed to update package visibility');
          this.notificationService.error('Failed to update package visibility', err.message);
          this._isLoading.set(false);
          this.loadingService.stop();
          return EMPTY;
        }),
        tap(() => this.loadingService.stop())
      )
      .subscribe(() => {
        this.notificationService.success(`Package visibility updated to ${visibility}`);
        this._isLoading.set(false);
        
        // Refresh package data
        if (this._currentPackage()) {
          this.loadPackage(ident);
        }
      });
  }

  /**
   * Resets the state
   */
  reset(): void {
    this._currentPackage.set(null);
    this._packageSearchResults.set(null);
    this._latestPackage.set(null);
    this._packageVersions.set([]);
    this._packageChannels.set([]);
    this._error.set(null);
    this._isLoading.set(false);
  }
}
