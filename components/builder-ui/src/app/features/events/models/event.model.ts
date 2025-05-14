// Event data models
export interface PackageIdent {
  origin: string;
  name: string;
  version: string;
  release: string;
}

export interface Event {
  operation: string;
  created_at: string;
  origin: string;
  channel: string;
  package_ident: PackageIdent;
}

export interface EventsResponse {
  range_start: number;
  range_end: number;
  total_count: number;
  data: Event[];
}

export interface EventsSearchParams {
  range?: number;
  channel?: string;
  from_date?: string;
  to_date?: string;
  query?: string;
}
