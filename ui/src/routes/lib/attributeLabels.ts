// Sentence-case display labels for the 10 Attributes, in the EXACT on-disk
// order from PLAN.md Appendix A: VE, RE, AG, CA, RM, RG, PA, TI, EN, PO.
// This is the single place the Squad/Player screen reads attribute order
// from — never iterate Object.keys(attrs) for display, since object key
// order isn't a format guarantee.
import type { Attributes } from "../../lib/model";

export interface AttributeLabel {
  key: keyof Attributes;
  code: string; // the two-letter code from Appendix A
  label: string; // sentence-case, shown in the UI
}

export const ATTRIBUTE_ORDER: AttributeLabel[] = [
  { key: "velocidad", code: "VE", label: "Speed" },
  { key: "resistencia", code: "RE", label: "Stamina" },
  { key: "agresividad", code: "AG", label: "Aggression" },
  { key: "calidad", code: "CA", label: "Quality" },
  { key: "remate", code: "RM", label: "Finishing" },
  { key: "regate", code: "RG", label: "Dribbling" },
  { key: "pase", code: "PA", label: "Passing" },
  { key: "tiro", code: "TI", label: "Shooting power" },
  { key: "entradas", code: "EN", label: "Tackling" },
  { key: "portero", code: "PO", label: "Goalkeeping" },
];
