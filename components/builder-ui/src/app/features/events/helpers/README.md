# Events UI Update Instructions

This folder contains the updated templates and styles for the events components in the application. Due to issues with the current components, we've provided these template files as a reference for implementing the updated UI.

## Implementation Steps

1. Fix the corrupted `events.component.ts` and `events-saas.component.ts` files:
   - Take a backup of the current files (already done as `.bak` files)
   - Create new files with proper imports and component structure
   - Copy the templates from the helper files into the new components

2. The main UI changes that need to be applied:
   - Use `<header>` with `<h1>` and `<h2>` for titles
   - Update component structure to match the design:
     ```
     <div class="events-component">
       <header>...</header>
       <div class="body">
         <div class="content">
           <section class="events-filter">...</section>
           <section>...</section>
           <section class="more">...</section>
         </div>
       </div>
     </div>
     ```
   - Add the "Load more" functionality at the bottom

3. The `loadMoreEvents()` method has already been added to the `BaseEventsComponent` class:
   ```typescript
   loadMoreEvents(): void {
     const prevPageSize = this.pageSize;
     this.pageSize += this.pageSize;
     
     // Reset current page since we're increasing page size instead
     const prevPage = this.currentPage;
     this.currentPage = 0;
     
     this.loadEvents();
   }
   ```

## Integrating with Existing Components

When rebuilding the components:

1. Make sure to maintain all the imports and Angular component decorators
2. Replace the template and styles with the ones from the helper files
3. Keep all the existing class methods and add the `loadMoreEvents` method if not inheriting from `BaseEventsComponent`
4. Update the component styles to match the new design

## Testing

After implementation:
1. Verify that both Events and Events (SaaS) pages load correctly
2. Test the search functionality
3. Test the date filter functionality 
4. Verify that the "Load more" link works properly
5. Ensure the UI matches the design specification
