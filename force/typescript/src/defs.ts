export interface Tag {
	name: string;
}

export type Bondee = {
	kind: 'choices',
	category: string[],
	family: string[]
} | { kind: 'all' };

export type DataType = {
	kind: 'bond',
	bondee: Bondee
} | {
	kind: 'tagged_bond',
	bondee: Bondee,
	tags: Tag[]
} | {
	kind: 'one_line'
} | {
	kind: 'text',
	regex: RegExp | undefined
} | {
	kind: 'number'
};

export interface Field {
	datatype: DataType,
	name: string
}

export interface Category {
	name: string,
	family: string[],
	fields: Field[]
}

export type Categories = Map<string, Category>;

export interface Force {
	families: Map<string, string[]>,
	categories: Categories
}
