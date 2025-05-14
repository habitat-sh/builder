import { Directive, OnInit, OnDestroy, inject } from '@angular/core';
import { FormBuilder, FormGroup, ReactiveFormsModule } from '@angular/forms';
import { Subscription, finalize } from 'rxjs';
import { MatSnackBar } from '@angular/material/snack-bar';
import { PageEvent } from '@angular/material/paginator';
import { DatePipe } from '@angular/common';

import { EventsService } from './services/events.service';
import { Event, EventsResponse, EventsSearchParams } from './models/event.model';

@Directive()
export abstract class BaseEventsComponent implements OnInit, OnDestroy {
  events: Event[] = [];
  loading = false;
  totalEvents = 0;
  pageSize = 20;
  currentPage = 0;
  
  searchForm: FormGroup;
  availableChannels = ['stable', 'unstable', 'dev'];
  
  protected subscriptions = new Subscription();
  protected eventsService = inject(EventsService);
  protected fb = inject(FormBuilder);
  protected snackBar = inject(MatSnackBar);

  constructor() {
    const dateRange = this.eventsService.getDefaultDateRange();
    
    this.searchForm = this.fb.group({
      channel: ['stable'],
      fromDate: [dateRange.from_date],
      toDate: [dateRange.to_date],
      query: ['']
    });
  }

  ngOnInit(): void {
    this.loadEvents();
  }

  ngOnDestroy(): void {
    this.subscriptions.unsubscribe();
  }

  /**
   * Search for events with current form values
   */
  onSearch(): void {
    this.currentPage = 0;
    this.loadEvents();
  }

  /**
   * Handle page change event
   */
  onPageChange(event: PageEvent): void {
    this.currentPage = event.pageIndex;
    this.pageSize = event.pageSize;
    this.loadEvents();
  }

  /**
   * Reset search form to default values
   */
  resetFilters(): void {
    const dateRange = this.eventsService.getDefaultDateRange();
    
    this.searchForm.patchValue({
      channel: 'stable',
      fromDate: dateRange.from_date,
      toDate: dateRange.to_date,
      query: ''
    });
    
    this.onSearch();
  }

  /**
   * Format a package identifier into a display string
   */
  formatPackageIdent(pkg: { origin: string, name: string, version: string, release: string }): string {
    return `${pkg.origin}/${pkg.name}/${pkg.version}/${pkg.release}`;
  }

  /**
   * Format date string into a more readable format
   */
  formatDate(dateString: string): string {
    const date = new Date(dateString);
    // Format: May 13, 2025, 2:30 PM
    return date.toLocaleDateString('en-US', { 
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit'
    });
  }

  /**
   * Load events from API
   * This is implemented by subclasses that specify which endpoint to use
   */
  protected abstract loadEvents(): void;
  
  /**
   * Build search params from form values
   */
  protected buildSearchParams(): EventsSearchParams {
    const formValues = this.searchForm.value;
    
    return {
      range: this.currentPage * this.pageSize,
      channel: formValues.channel,
      from_date: formValues.fromDate,
      to_date: formValues.toDate,
      query: formValues.query
    };
  }

  /**
   * Handle events response
   */
  protected handleEventsResponse(response: EventsResponse): void {
    this.events = response.data;
    this.totalEvents = response.total_count;
  }

  /**
   * Handle API error
   */
  protected handleError(error: any): void {
    console.error('Error fetching events', error);
    this.snackBar.open('Failed to load events. Please try again.', 'Dismiss', {
      duration: 5000
    });
  }

  /**
   * Load more events when clicking "Load more" button
   */
  loadMoreEvents(): void {
    const prevPageSize = this.pageSize;
    this.pageSize += this.pageSize;
    
    // Reset current page since we're increasing page size instead
    const prevPage = this.currentPage;
    this.currentPage = 0;
    
    this.loadEvents();
  }
}
