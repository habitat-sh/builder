import { Component, EventEmitter, Input, Output, OnChanges, SimpleChanges } from '@angular/core';
import { CommonModule } from '@angular/common';
import { MatPaginatorModule, PageEvent } from '@angular/material/paginator';
import { MatSelectModule } from '@angular/material/select';
import { FormsModule } from '@angular/forms';

export interface PaginationOptions {
  page: number;         // Current page (0-based)
  pageSize: number;     // Items per page
  totalItems: number;   // Total number of items
}

@Component({
  selector: 'app-pagination',
  standalone: true,
  imports: [CommonModule, MatPaginatorModule, MatSelectModule, FormsModule],
  template: `
    <div class="pagination-container">
      <div class="pagination-info" *ngIf="showPageInfo">
        Showing {{ startItem }}-{{ endItem }} of {{ totalItems }}
      </div>
      
      <mat-paginator
        [length]="totalItems"
        [pageSize]="pageSize"
        [pageSizeOptions]="pageSizeOptions"
        [pageIndex]="currentPage"
        (page)="onPageChange($event)"
        [showFirstLastButtons]="showFirstLastButtons"
        aria-label="Select page">
      </mat-paginator>
    </div>
  `,
  styles: [`
    .pagination-container {
      display: flex;
      align-items: center;
      justify-content: space-between;
      flex-wrap: wrap;
      padding: 8px;
    }
    
    .pagination-info {
      color: rgba(0, 0, 0, 0.6);
      font-size: 14px;
      margin-right: 16px;
    }
    
    @media (max-width: 600px) {
      .pagination-container {
        flex-direction: column;
        align-items: stretch;
      }
      
      .pagination-info {
        margin-bottom: 8px;
        text-align: center;
      }
    }
  `]
})
export class PaginationComponent implements OnChanges {
  @Input() totalItems = 0;
  @Input() pageSize = 10;
  @Input() currentPage = 0;
  @Input() pageSizeOptions: number[] = [5, 10, 25, 50, 100];
  @Input() showPageInfo = true;
  @Input() showFirstLastButtons = true;
  
  @Output() pageChange = new EventEmitter<PaginationOptions>();
  
  startItem = 1;
  endItem = 0;
  
  ngOnChanges(changes: SimpleChanges): void {
    this.updateDisplayedItemRange();
  }
  
  onPageChange(event: PageEvent): void {
    this.currentPage = event.pageIndex;
    this.pageSize = event.pageSize;
    this.updateDisplayedItemRange();
    
    this.pageChange.emit({
      page: this.currentPage,
      pageSize: this.pageSize,
      totalItems: this.totalItems
    });
  }
  
  private updateDisplayedItemRange(): void {
    this.startItem = this.currentPage * this.pageSize + 1;
    this.endItem = Math.min(this.startItem + this.pageSize - 1, this.totalItems);
    
    // Handle empty results
    if (this.totalItems === 0) {
      this.startItem = 0;
      this.endItem = 0;
    }
  }
}
