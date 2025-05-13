import { ComponentFixture, TestBed, fakeAsync, tick } from '@angular/core/testing';
import { RouterTestingModule } from '@angular/router/testing';
import { HttpClientTestingModule } from '@angular/common/http/testing';
import { ReactiveFormsModule } from '@angular/forms';
import { NoopAnimationsModule } from '@angular/platform-browser/animations';
import { ActivatedRoute, Router, convertToParamMap } from '@angular/router';
import { Title } from '@angular/platform-browser';
import { of } from 'rxjs';

import { SearchComponent } from './search.component';
import { PackageSearchService } from './services/package-search.service';

describe('SearchComponent', () => {
  let component: SearchComponent;
  let fixture: ComponentFixture<SearchComponent>;
  let mockSearchService: jasmine.SpyObj<PackageSearchService>;
  let mockRouter: any;
  let mockTitleService: jasmine.SpyObj<Title>;

  beforeEach(async () => {
    mockSearchService = jasmine.createSpyObj('PackageSearchService', ['searchPackages']);
    mockSearchService.searchPackages.and.returnValue(of({
      results: [],
      totalCount: 0,
      nextRange: 0
    }));
    
    mockTitleService = jasmine.createSpyObj('Title', ['setTitle']);

    await TestBed.configureTestingModule({
      imports: [
        RouterTestingModule,
        HttpClientTestingModule,
        ReactiveFormsModule,
        NoopAnimationsModule,
        SearchComponent
      ],
      providers: [
        {
          provide: ActivatedRoute,
          useValue: {
            params: of({ q: 'test', origin: 'core' }),
            snapshot: {
              params: { q: 'test', origin: 'core' },
              queryParamMap: convertToParamMap({})
            }
          }
        },
        { provide: PackageSearchService, useValue: mockSearchService },
        { provide: Title, useValue: mockTitleService }
      ]
    }).compileComponents();

    fixture = TestBed.createComponent(SearchComponent);
    component = fixture.componentInstance;
    mockRouter = TestBed.inject(Router);
    spyOn(mockRouter, 'navigate').and.returnValue(Promise.resolve(true));
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should initialize search with route params', () => {
    expect(mockSearchService.searchPackages).toHaveBeenCalledWith('core', 'test', 0);
    expect(mockTitleService.setTitle).toHaveBeenCalledWith('Search › core › test › Results | Habitat Builder');
  });
  
  it('should change origin and update search results', () => {
    mockSearchService.searchPackages.calls.reset();
    component.changeOrigin('chef');
    expect(mockSearchService.searchPackages).toHaveBeenCalledWith('chef', 'test', 0);
    expect(mockRouter.navigate).toHaveBeenCalledWith(['/search', { q: 'test', origin: 'chef' }]);
  });
  
  it('should handle search submission', () => {
    component.submit('nginx');
    expect(mockRouter.navigate).toHaveBeenCalledWith(['/search', { q: 'nginx', origin: 'core' }]);
  });
  
  it('should fetch more packages when loadMore is called', () => {
    // Set up component with some existing results
    component['_packages'].set([{ id: '1', ident: { origin: 'core', name: 'nginx' }} as any]);
    component['_totalCount'].set(100);
    
    mockSearchService.searchPackages.calls.reset();
    mockSearchService.searchPackages.and.returnValue(of({
      results: [{ id: '2', ident: { origin: 'core', name: 'redis' }} as any],
      totalCount: 100,
      nextRange: 50
    }));
    
    component.fetchMorePackages();
    expect(mockSearchService.searchPackages).toHaveBeenCalledWith('core', 'test', 1);
  });
  
  it('should handle search input changes with debounce', fakeAsync(() => {
    mockSearchService.searchPackages.calls.reset();
    component.searchBox.setValue('redis');
    
    // Should not trigger immediate search
    expect(mockSearchService.searchPackages).not.toHaveBeenCalled();
    
    // After debounce time (400ms)
    tick(500);
    expect(mockSearchService.searchPackages).toHaveBeenCalledWith('core', 'redis', 0);
  }));
});
