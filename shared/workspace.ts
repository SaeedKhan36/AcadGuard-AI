export type DocumentItem = {
  id: string;
  name: string;
  course: string;
  author: string;
  initials: string;
  date: string;
  similarity: number;
  ai: number;
  status: string;
  words: number;
};
export const demoDocuments: DocumentItem[] = [
  {
    id: "sample-1",
    name: "The changing landscape of urban ecology",
    course: "Environmental Studies",
    author: "Alex Lewis",
    initials: "AL",
    date: "Oct 23, 2026",
    similarity: 8,
    ai: 0,
    status: "Reviewed",
    words: 2450,
  },
  {
    id: "sample-2",
    name: "Ethics in the age of artificial intelligence",
    course: "Philosophy & Ethics",
    author: "Maya Patel",
    initials: "MP",
    date: "Oct 23, 2026",
    similarity: 14,
    ai: 22,
    status: "Needs review",
    words: 3180,
  },
  {
    id: "sample-3",
    name: "Rethinking sustainable economic growth",
    course: "Environmental Studies",
    author: "James Wilson",
    initials: "JW",
    date: "Oct 22, 2026",
    similarity: 5,
    ai: 0,
    status: "Reviewed",
    words: 2800,
  },
  {
    id: "sample-4",
    name: "The language of belonging",
    course: "Academic Writing",
    author: "Sofia Chen",
    initials: "SC",
    date: "Oct 22, 2026",
    similarity: 27,
    ai: 0,
    status: "Needs review",
    words: 1920,
  },
  {
    id: "sample-5",
    name: "Community-led approaches to conservation",
    course: "Environmental Studies",
    author: "Noah Martin",
    initials: "NM",
    date: "Oct 21, 2026",
    similarity: 11,
    ai: 0,
    status: "Reviewed",
    words: 2240,
  },
  {
    id: "sample-6",
    name: "A critical reading of modern narratives",
    course: "Academic Writing",
    author: "Emma Davis",
    initials: "ED",
    date: "Oct 21, 2026",
    similarity: 6,
    ai: 0,
    status: "Reviewed",
    words: 2640,
  },
];

export type WorkspacePreferences = {
  name: string;
  role: string;
  institution: string;
  language: string;
  reminders: boolean;
  resources: boolean;
};
