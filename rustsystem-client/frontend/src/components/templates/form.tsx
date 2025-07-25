import React, { useEffect, useState } from "react";
import '@/colors.css';
import Button from "@/components/templates/button";

interface Field {
  label: string;
  id: string;
  type: string;
}

interface Submit {
  label: string,
  data: (data: Record<string, string>) => void;
}

interface FormSectionProps {
  fields: Field[];
  submit: Submit
}

const FormSection: React.FC<FormSectionProps> = ({ fields, submit }) => {
  console.log("FormSection rendered with fields:", fields);
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
        className="bg-[rgba(255,255,255,0.05)] backdrop-blur-sm border border-[var(--color-contours)] rounded-lg p-8 shadow-lg space-y-6"
      >
        {fields.map((field) => (
          <div key={field.id}>
            <label
              className="block text-sm mb-2 opacity-80"
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
              className="w-full p-3 rounded-lg bg-transparent border border-[var(--color-contours)] focus:outline-none focus:ring-2 focus:ring-[var(--color-main)] transition"
            />
          </div>
        ))}
        <Button label={submit.label} fn={handleSubmit} />
      </form>
    </section>
  );
};

export default FormSection;

