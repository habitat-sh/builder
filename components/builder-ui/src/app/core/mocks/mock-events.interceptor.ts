import { Injectable } from '@angular/core';
import { HttpRequest, HttpResponse, HttpHandler, HttpEvent, HttpInterceptor } from '@angular/common/http';
import { Observable, of } from 'rxjs';
import { delay } from 'rxjs/operators';
import { EventsResponse } from '../../features/events/models/event.model';

@Injectable()
export class MockEventsInterceptor implements HttpInterceptor {
  intercept(request: HttpRequest<any>, next: HttpHandler): Observable<HttpEvent<any>> {
    // Only intercept requests to the events API endpoints
    if (request.url.includes('/v1/depot/events')) {
      // Get query parameters
      const urlParts = request.url.split('?');
      const queryParams = new URLSearchParams(urlParts.length > 1 ? urlParts[1] : '');
      const channel = queryParams.get('channel') || 'stable';
      const query = queryParams.get('query') || '';
      
      let mockData: EventsResponse;
      
      // Check if this is a request for SaaS events
      const isSaasRequest = request.url.includes('/events/saas');
      
      if (isSaasRequest) {
        mockData = this.generateMockSaasEvents(query, channel);
      } else {
        mockData = this.generateMockEvents(query, channel);
      }
      
      // Return a mock response after a small delay to simulate network latency
      return of(new HttpResponse({ 
        status: 200, 
        body: mockData 
      })).pipe(delay(500));
    }
    
    // Pass through any requests not handled above
    return next.handle(request);
  }
  
  private generateMockEvents(query: string, channel: string): EventsResponse {
    const mockEvents = [];
    
    // Generate 20 mock events
    for (let i = 0; i < 20; i++) {
      mockEvents.push({
        operation: i % 3 === 0 ? 'upload' : i % 2 === 0 ? 'promote' : 'download',
        created_at: new Date(Date.now() - i * 3600 * 1000).toISOString(),
        origin: 'core',
        channel: channel,
        package_ident: {
          origin: 'core',
          name: 'nginx',
          version: '1.19.' + i,
          release: '20250' + i
        }
      });
    }
    
    // If there's a search query, filter the results
    let filteredEvents = mockEvents;
    if (query) {
      filteredEvents = mockEvents.filter(event => 
        event.package_ident.name.includes(query) || 
        event.operation.includes(query) ||
        event.origin.includes(query)
      );
    }
    
    return {
      range_start: 0,
      range_end: filteredEvents.length - 1,
      total_count: filteredEvents.length * 2, // Simulate more data for pagination/load more
      data: filteredEvents
    };
  }
  
  private generateMockSaasEvents(query: string, channel: string): EventsResponse {
    const mockEvents = [];
    
    // Generate 20 mock SaaS events
    for (let i = 0; i < 20; i++) {
      mockEvents.push({
        operation: i % 3 === 0 ? 'deploy' : i % 2 === 0 ? 'rollback' : 'scale',
        created_at: new Date(Date.now() - i * 3600 * 1000).toISOString(),
        origin: 'saas-' + (i % 3),
        channel: channel,
        package_ident: {
          origin: 'saas-' + (i % 3),
          name: 'service-' + (i % 5),
          version: '2.0.' + i,
          release: '20250' + i
        }
      });
    }
    
    // If there's a search query, filter the results
    let filteredEvents = mockEvents;
    if (query) {
      filteredEvents = mockEvents.filter(event => 
        event.package_ident.name.includes(query) || 
        event.operation.includes(query) ||
        event.origin.includes(query)
      );
    }
    
    return {
      range_start: 0,
      range_end: filteredEvents.length - 1,
      total_count: filteredEvents.length * 2, // Simulate more data for pagination/load more
      data: filteredEvents
    };
  }
}
