import { Injectable } from '@angular/core';
import { Observable } from 'rxjs';
import { EventsResponse, EventsSearchParams } from '../models/event.model';
import { formatDate } from '@angular/common';
import { ApiService } from '../../../core/services/api.service';

@Injectable({
  providedIn: 'root'
})
export class EventsService {
  constructor(private apiService: ApiService) { }

  /**
   * Get regular events
   */
  getEvents(params: EventsSearchParams): Observable<EventsResponse> {
    const queryParams = this.buildQueryParams(params);
    return this.apiService.get<EventsResponse>(`v1/depot/events${queryParams}`);
  }

  /**
   * Get SaaS events
   */
  getSaasEvents(params: EventsSearchParams): Observable<EventsResponse> {
    const queryParams = this.buildQueryParams(params);
    return this.apiService.get<EventsResponse>(`v1/depot/events/saas${queryParams}`);
  }

  /**
   * Build query parameters string from search params
   */
  private buildQueryParams(params: EventsSearchParams): string {
    const queryParams: string[] = [];
    
    if (params.range !== undefined) {
      queryParams.push(`range=${params.range}`);
    }
    
    if (params.channel) {
      queryParams.push(`channel=${params.channel}`);
    }
    
    if (params.from_date) {
      queryParams.push(`from_date=${params.from_date}`);
    }
    
    if (params.to_date) {
      queryParams.push(`to_date=${params.to_date}`);
    }
    
    if (params.query) {
      queryParams.push(`query=${encodeURIComponent(params.query)}`);
    }
    
    return queryParams.length ? `?${queryParams.join('&')}` : '';
  }

  /**
   * Get the default date range for the last 7 days
   */
  getDefaultDateRange(): { from_date: string; to_date: string } {
    const toDate = new Date();
    const fromDate = new Date();
    fromDate.setDate(fromDate.getDate() - 7);
    
    return {
      from_date: formatDate(fromDate, 'yyyy-MM-dd', 'en-US'),
      to_date: formatDate(toDate, 'yyyy-MM-dd', 'en-US')
    };
  }
}
