import type React from "react";
import { useEffect, useState } from "react";
import "@/colors.css";
import Button from "@/components/templates/button";

interface Field {
	label: string;
	id: string;
	type: string;
}

interface Submit {
	label: string;
	data: (data: Record<string, string>) => void;
}

interface FormSectionProps {
	fields: Field[];
	submit: Submit;
}

const FormSection: React.FC<FormSectionProps> = ({ fields, submit }) => {
	const [formData, setFormData] = useState<Record<string, string>>({});

	useEffect(() => {
		setFormData(Object.fromEntries(fields.map((f) => [f.id, ""])));
	}, []);

	const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
		const { id, value } = e.target;
		setFormData((prev) => ({ ...prev, [id]: value }));
	};

	const handleSubmit = (e: React.FormEvent) => {
		e.preventDefault();
		submit.data(formData);
	};

	return (
		<section className="container mx-auto px-4 mt-12 max-w-3xl">
			<form
				onSubmit={handleSubmit}
				className="bg-white border border-gray-200 rounded-lg p-8 shadow-sm space-y-6"
			>
				{fields.map((field) => (
					<div key={field.id}>
						<label
							className="block text-sm mb-2 text-gray-700 font-medium"
							htmlFor={field.id}
						>
							{field.label}
						</label>
						<input
							id={field.id}
							type={field.type}
							required
							value={formData[field.id]}
							onChange={handleChange}
							className="w-full p-3 rounded border border-gray-300 focus:outline-none focus:ring-2 focus:ring-[var(--color-main)] focus:border-transparent transition-all duration-100"
						/>
					</div>
				))}
				<Button label={submit.label} fn={handleSubmit} />
			</form>
		</section>
	);
};

export default FormSection;
