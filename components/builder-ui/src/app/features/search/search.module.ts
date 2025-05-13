import { NgModule } from '@angular/core';
import { SearchRoutingModule } from './search-routing.module';
import { SearchComponent } from './search.component';
import { SearchResultsComponent } from './search-results/search-results.component';
import { PackageSearchService } from './services/package-search.service';

@NgModule({
  imports: [
    SearchRoutingModule,
    SearchComponent,
    SearchResultsComponent
  ],
  providers: [
    PackageSearchService
  ]
})
export class SearchModule { }
