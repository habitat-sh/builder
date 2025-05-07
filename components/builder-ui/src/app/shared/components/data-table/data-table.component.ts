import { Component, Input, Output, EventEmitter, ViewChild, OnChanges, SimpleChanges } from '@angular/core';
import { CommonModule } from '@angular/common';
import { MatTableModule, MatTableDataSource } from '@angular/material/table';
import { MatPaginator, MatPaginatorModule } from '@angular/material/paginator';
import { MatSort, MatSortModule } from '@angular/material/sort';
import { MatInputModule } from '@angular/material/input';
import { MatFormFieldModule } from '@angular/material/form-field';
import { FormsModule } from '@angular/forms';

export interface Column {
  name: string;       // Property name in the data object
  label: string;      // Display label for the column
  sortable?: boolean; // Whether the column is sortable
  format?: (value: any) => string; // Optional formatter function
  width?: string;     // Optional width specification (e.g., '100px', '10%')
}

@Component({
  selector: 'app-data-table',
  standalone: true,
  imports: [
    CommonModule, 
    MatTableModule, 
    MatPaginatorModule, 
    MatSortModule, 
    MatInputModule,
    MatFormFieldModule,
    FormsModule
  ],
  template: `
    <div class="table-container">
      <mat-form-field *ngIf="showFilter" class="filter-field">
        <mat-label>Filter</mat-label>
        <input matInput (keyup)="applyFilter($event)" placeholder="Type to filter..." #input>
      </mat-form-field>

      <div class="table-wrapper">
        <table mat-table [dataSource]="dataSource" matSort>
          <!-- Dynamic columns -->
          <ng-container *ngFor="let column of columns" [matColumnDef]="column.name">
            <th mat-header-cell *matHeaderCellDef [mat-sort-header]="column.sortable ? column.name : null" 
                [style.width]="column.width">
              {{ column.label }}
            </th>
            <td mat-cell *matCellDef="let element">
              {{ column.format ? column.format(element[column.name]) : element[column.name] }}
            </td>
          </ng-container>

          <!-- Actions column -->
          <ng-container *ngIf="showActions" matColumnDef="actions">
            <th mat-header-cell *matHeaderCellDef>Actions</th>
            <td mat-cell *matCellDef="let element">
              <ng-content [select]="'[actions]'" [ngTemplateOutletContext]="{ $implicit: element }"></ng-content>
            </td>
          </ng-container>

          <tr mat-header-row *matHeaderRowDef="displayedColumns; sticky: stickyHeader"></tr>
          <tr 
            mat-row 
            *matRowDef="let row; columns: displayedColumns;" 
            (click)="onRowClick(row)" 
            [class.clickable]="isRowClickable"
          ></tr>

          <!-- Row displayed when there is no matching data -->
          <tr class="mat-row" *matNoDataRow>
            <td class="mat-cell no-data-cell" [attr.colspan]="displayedColumns.length">
              {{ noDataMessage }}
            </td>
          </tr>
        </table>
      </div>

      <mat-paginator 
        *ngIf="showPaginator"
        [pageSizeOptions]="pageSizeOptions" 
        [pageSize]="pageSize"
        showFirstLastButtons 
        aria-label="Select page">
      </mat-paginator>
    </div>
  `,
  styles: [`
    .table-container {
      width: 100%;
      overflow: auto;
    }

    .filter-field {
      width: 100%;
      margin-bottom: 12px;
    }

    .table-wrapper {
      overflow: auto;
      max-height: 70vh;
    }

    table {
      width: 100%;
      min-width: 500px;
    }

    .clickable {
      cursor: pointer;
    }

    .clickable:hover {
      background-color: rgba(0, 0, 0, 0.04);
    }

    .no-data-cell {
      text-align: center;
      padding: 16px;
      font-style: italic;
      color: rgba(0, 0, 0, 0.54);
    }

    th.mat-header-cell {
      font-weight: bold;
      color: rgba(0, 0, 0, 0.87);
    }

    .mat-column-actions {
      width: 100px;
      text-align: center;
    }
  `]
})
export class DataTableComponent implements OnChanges {
  @Input() columns: Column[] = [];
  @Input() data: any[] = [];
  @Input() showFilter = true;
  @Input() showPaginator = true;
  @Input() pageSizeOptions: number[] = [5, 10, 25, 100];
  @Input() pageSize = 10;
  @Input() stickyHeader = false;
  @Input() showActions = false;
  @Input() isRowClickable = false;
  @Input() noDataMessage = 'No data found';

  @Output() rowClick = new EventEmitter<any>();

  @ViewChild(MatPaginator) paginator!: MatPaginator;
  @ViewChild(MatSort) sort!: MatSort;

  dataSource = new MatTableDataSource<any>([]);
  displayedColumns: string[] = [];

  ngOnChanges(changes: SimpleChanges): void {
    if (changes['data'] || changes['columns']) {
      this.setupTable();
    }
  }

  ngAfterViewInit(): void {
    this.dataSource.paginator = this.paginator;
    this.dataSource.sort = this.sort;
  }

  setupTable(): void {
    // Set data
    this.dataSource.data = this.data || [];

    // Set columns
    this.displayedColumns = this.columns.map(column => column.name);
    if (this.showActions) {
      this.displayedColumns.push('actions');
    }
  }

  applyFilter(event: Event): void {
    const filterValue = (event.target as HTMLInputElement).value;
    this.dataSource.filter = filterValue.trim().toLowerCase();

    if (this.dataSource.paginator) {
      this.dataSource.paginator.firstPage();
    }
  }

  onRowClick(row: any): void {
    if (this.isRowClickable) {
      this.rowClick.emit(row);
    }
  }
}
