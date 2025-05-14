import { Component, OnInit, OnDestroy, inject, signal, computed, effect } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormControl, ReactiveFormsModule } from '@angular/forms';
import { Title } from '@angular/platform-browser';
import { ActivatedRoute, Router } from '@angular/router';
import { HeaderService } from '../../core/services/header.service';
import { HeaderTitleDirective } from '../../core/layout/shared';
import { MatInputModule } from '@angular/material/input';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { MatCardModule } from '@angular/material/card';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';
import { MatSelectModule } from '@angular/material/select';
import { debounceTime, distinctUntilChanged, Subscription } from 'rxjs';
import { SearchResultsComponent } from './search-results/search-results.component';
import { PackageSearchService, PackageSearchResponse } from './services/package-search.service';
import { Package, PackageIdent } from '../../shared/models/package.model';

@Component({
  selector: 'app-search',
  standalone: true,
  imports: [
    CommonModule,
    ReactiveFormsModule,
    MatInputModule,
    MatButtonModule,
    MatIconModule,
    MatCardModule,
    MatProgressSpinnerModule,
    MatSelectModule,
    SearchResultsComponent,
    HeaderTitleDirective
  ],
  template: `
    <!-- Header Title Template -->
    <ng-template habHeaderTitle>
      <h1>Search Packages</h1>
      <h2 *ngIf="searchQuery()">Search Results</h2>
    </ng-template>
    
    <div class="search-container">
      <div class="search-body">
        <section class="search-input-section">
          <div class="search-form">
            <mat-form-field appearance="outline" class="search-input">
              <mat-label>Search Packages</mat-label>
              <input 
                matInput 
                type="search" 
                [formControl]="searchBox" 
                placeholder="Search for packages..."
                (keyup.enter)="submit(searchBox.value)"
                aria-label="Enter search keywords"
                id="package-search">
              <mat-icon matSuffix aria-hidden="true">search</mat-icon>
            </mat-form-field>
            
            <mat-form-field appearance="outline" class="origin-select">
              <mat-label>Origin</mat-label>
              <mat-select 
                [formControl]="originSelect" 
                (selectionChange)="changeOrigin($event.value)"
                aria-label="Select package origin"
                id="origin-select">
                <mat-option value="*">All Origins</mat-option>
                <mat-option value="core">Core</mat-option>
                <mat-option *ngFor="let origin of popularOrigins" [value]="origin">
                  {{ origin }}
                </mat-option>
              </mat-select>
            </mat-form-field>
          </div>
        </section>
        
        <div *ngIf="isLoading()" class="loading-spinner" role="status" aria-live="polite">
          <mat-spinner diameter="40"></mat-spinner>
          <span>Loading packages...</span>
        </div>
        
        <div *ngIf="error()" class="search-error" role="alert" aria-live="assertive">
          <mat-icon color="warn">error_outline</mat-icon>
          <h3>Error Loading Packages</h3>
          <p>{{ error() }}</p>
        </div>
        
        <app-search-results 
          [packages]="packages()"
          [noPackages]="packages().length === 0 && !isLoading() && !error()"
          [isLoading]="isLoading()">
        </app-search-results>
        
        <section class="load-more" *ngIf="packages().length < totalCount() && packages().length > 0">
          <p>Showing {{packages().length}} of {{totalCount()}} packages.</p>
          <button mat-stroked-button color="primary" (click)="fetchMorePackages()" [disabled]="isLoading()">
            <span *ngIf="!isLoading()">Load {{(totalCount() - packages().length) > perPage ? perPage : totalCount() - packages().length }} more</span>
            <span *ngIf="isLoading()">Loading...</span>
          </button>
        </section>
      </div>
    </div>
  `,
  styles: [`
    .search-container {
      padding: 20px;
      max-width: 1200px;
      margin: 0 auto;
    }

    h2 strong {
      color: #1976d2;
    }
    
    .search-input-section {
      margin-bottom: 24px;
    }

    .search-form {
      display: flex;
      flex-wrap: wrap;
      gap: 16px;
      align-items: flex-start;
    }
    
    .search-input {
      flex: 1;
      min-width: 300px;
    }

    .origin-select {
      width: 200px;
    }
    
    .loading-spinner {
      display: flex;
      flex-direction: column;
      align-items: center;
      padding: 48px 0;
      text-align: center;
    }
    
    .loading-spinner span {
      margin-top: 16px;
      color: #666;
    }
    
    .search-error {
      background-color: #fff8f7;
      border-left: 4px solid #d32f2f;
      padding: 16px;
      margin: 16px 0;
      border-radius: 4px;
      text-align: center;
    }
    
    .search-error mat-icon {
      font-size: 24px;
      height: 24px;
      width: 24px;
      margin-bottom: 8px;
    }
    
    .search-error h3 {
      margin: 0 0 8px 0;
      color: #d32f2f;
    }
    
    .search-error p {
      margin: 0;
      color: #666;
    }
    
    .load-more {
      margin-top: 24px;
      text-align: center;
      padding: 16px;
      border-top: 1px solid #eee;
      cursor: pointer;
    }
    
    .load-more p {
      margin: 0 0 12px 0;
      color: #666;
    }
    
    .load-more button {
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }
  `]
})
export class SearchComponent implements OnInit, OnDestroy {
  private route = inject(ActivatedRoute);
  private router = inject(Router);
  private title = inject(Title);
  private searchService = inject(PackageSearchService);
  headerService = inject(HeaderService); // Make public for template access
  
  // Form controls
  searchBox = new FormControl('');
  originSelect = new FormControl('core');
  
  // Popular origins (could be fetched from API in a real implementation)
  popularOrigins = ['chef', 'habitat', 'uname', 'bldr'];
  
  // State signals
  private _searchQuery = signal<string>('');
  private _packages = signal<Package[]>([]);
  private _isLoading = signal<boolean>(false);
  private _error = signal<string | null>(null);
  private _totalCount = signal<number>(0);
  private _originSignal = signal<string>('core');
  
  // Computed values
  readonly searchQuery = computed(() => this._searchQuery());
  readonly packages = computed(() => this._packages());
  readonly isLoading = computed(() => this._isLoading());
  readonly error = computed(() => this._error());
  readonly totalCount = computed(() => this._totalCount());
  readonly origin = computed(() => this._originSignal());
  readonly perPage = 50;
  
  private subscriptions = new Subscription();
  
  ngOnInit(): void {
    // First check route params
    this.subscriptions.add(
      this.route.params.subscribe(params => {
        const query = params['q'] || '';
        const origin = params['origin'] || 'core';
        
        this._originSignal.set(origin);
        this.originSelect.setValue(origin, { emitEvent: false });
        
        if (query) {
          this._searchQuery.set(query);
          this.searchBox.setValue(query, { emitEvent: false });
          this.title.setTitle(`Search › ${origin} › ${query} › Results | Habitat Builder`);
        } else {
          this.title.setTitle(`Packages › ${origin} | Habitat Builder`);
        }
        
        this.fetchPackages();
      })
    );
    
    // Then check query params (used with /pkgs route)
    this.subscriptions.add(
      this.route.queryParams.subscribe(params => {
        // Only process if we have query parameters and no route params
        if (Object.keys(params).length > 0 && !this.route.snapshot.params['q']) {
          const query = params['q'] || '';
          const origin = params['origin'] || this._originSignal();
          
          if (origin !== this._originSignal()) {
            this._originSignal.set(origin);
            this.originSelect.setValue(origin, { emitEvent: false });
          }
          
          if (query && query !== this._searchQuery()) {
            this._searchQuery.set(query);
            this.searchBox.setValue(query, { emitEvent: false });
            this.title.setTitle(`Search › ${origin} › ${query} › Results | Habitat Builder`);
            this.fetchPackages();
          }
        }
      })
    );
    
    // Subscribe to search box changes for real-time search
    this.subscriptions.add(
      this.searchBox.valueChanges.pipe(
        debounceTime(400),
        distinctUntilChanged()
      ).subscribe(query => {
        if (query === null || query === undefined) return;
        
        if (!query.trim()) {
          this.router.navigate(['/pkgs']);
          return;
        }
        
        this._searchQuery.set(query);
        this.fetchPackages();
      })
    );
  }
  
  ngOnDestroy(): void {
    this.subscriptions.unsubscribe();
  }
  
  fetchPackages(): void {
    this._isLoading.set(true);
    this._error.set(null);
    
    // Update URL to reflect the search based on which route we're on
    if (this._searchQuery()) {
      const currentUrl = this.router.url;
      
      if (currentUrl.startsWith('/pkgs') && !currentUrl.includes(';')) {
        // If we're on /pkgs route with query params
        const params = {
          q: this._searchQuery(),
          origin: this._originSignal()
        };
        
        // Update URL without triggering navigation
        this.router.navigate([], {
          relativeTo: this.route,
          queryParams: params,
          queryParamsHandling: 'merge',
          replaceUrl: true
        });
      } else if (currentUrl.startsWith('/search') || currentUrl.includes(';')) {
        // If we're on /search route with matrix params, don't change the URL here
        // The matrix params are handled in submit() and changeOrigin()
      }
    }
    
    this.searchService.searchPackages(
      this._originSignal(), 
      this._searchQuery(), 
      0
    ).subscribe({
      next: (response) => {
        this._packages.set(response.results);
        this._totalCount.set(response.totalCount);
        this._isLoading.set(false);
        
        // Update title with search information
        if (this._searchQuery()) {
          this.title.setTitle(`Search › ${this._originSignal()} › ${this._searchQuery()} › ${response.results.length} of ${response.totalCount} Results | Habitat Builder`);
        } else {
          this.title.setTitle(`Packages › ${this._originSignal()} | Habitat Builder`);
        }
      },
      error: (err) => {
        console.error('Error fetching packages:', err);
        this._error.set('Failed to load packages. Please try again later.');
        this._isLoading.set(false);
      }
    });
  }
  
  fetchMorePackages(): void {
    if (this._isLoading()) return;
    
    this._isLoading.set(true);
    this._error.set(null);
    
    // Calculate the next range based on current packages length
    const nextRange = this._packages().length;
    
    this.searchService.searchPackages(
      this._originSignal(), 
      this._searchQuery(), 
      nextRange
    ).subscribe({
      next: (response) => {
        // Append new results to existing packages
        this._packages.update(packages => [...packages, ...response.results]);
        this._totalCount.set(response.totalCount);
        this._isLoading.set(false);
        
        // Update the title with the latest counts for screen reader users
        this.title.setTitle(`Search › ${this._originSignal()} › ${this._searchQuery()} › ${this._packages().length} of ${response.totalCount} Results | Habitat Builder`);
      },
      error: (err) => {
        console.error('Error fetching more packages:', err);
        this._error.set('Failed to load more packages. Please try again later.');
        this._isLoading.set(false);
      }
    });
  }
  
  submit(query: string | null): void {
    if (!query) return;
    
    const trimmedQuery = query.trim();
    if (trimmedQuery) {
      // Determine which route pattern to use based on the current URL
      const currentUrl = this.router.url;
      
      if (currentUrl.startsWith('/pkgs')) {
        // If we're on /pkgs route, use query params
        this.router.navigate(['/pkgs'], { 
          queryParams: { q: trimmedQuery, origin: this._originSignal() } 
        });
      } else {
        // Default to /search with matrix params for backward compatibility
        this.router.navigate(['/search', { q: trimmedQuery, origin: this._originSignal() }]);
      }
    } else {
      // If the query is empty, navigate to packages page
      this.router.navigate(['/pkgs']);
    }
  }
  
  /**
   * Change the origin filter and reload packages
   */
  changeOrigin(origin: string): void {
    this._originSignal.set(origin);
    this.fetchPackages();
    
    // Determine which route pattern to use based on the current URL
    const currentUrl = this.router.url;
    
    if (currentUrl.startsWith('/pkgs')) {
      // If we're on /pkgs route, use query params
      if (this._searchQuery()) {
        this.router.navigate(['/pkgs'], { 
          queryParams: { q: this._searchQuery(), origin }
        });
      } else {
        this.router.navigate(['/pkgs'], { 
          queryParams: { origin }
        });
      }
    } else {
      // Default to /search with matrix params
      if (this._searchQuery()) {
        this.router.navigate(['/search', { q: this._searchQuery(), origin }]);
      } else {
        this.router.navigate(['/search', { origin }]);
      }
    }
  }
}
