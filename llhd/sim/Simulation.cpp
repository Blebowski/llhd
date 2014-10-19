/* Copyright (c) 2014 Fabian Schuiki */
#include "llhd/Assembly.hpp"
#include "llhd/sim/Simulation.hpp"
#include <iostream>
#include <stdexcept>
using namespace llhd;

Simulation::Simulation(const AssemblyModule& as):
	as(as) {

	// Initialize the signal wrappers.
	for (auto& is : as.signals) {
		const auto s = is.second.get();

		std::cout << "wrapping " << s->name << '\n';
		wrappers[s].reset(new SimulationSignal(s,
			getInitialValue(s->type.get())
		));
	}
}

SimulationValue Simulation::getInitialValue(const AssemblyType* type) {
	if (dynamic_cast<const AssemblyTypeLogic*>(type)) {
		return SimulationValue(1, kLogicU);
	}
	if (auto subtype = dynamic_cast<const AssemblyTypeWord*>(type)) {
		if (dynamic_cast<const AssemblyTypeLogic*>(subtype->type.get())) {
			return SimulationValue(subtype->width, kLogicU);
		}
	}
	throw std::runtime_error("unknown type");
}

bool Simulation::observe(const AssemblySignal* signal) {
	auto it = wrappers.find(signal);
	if (it == wrappers.end())
		return false;

	observedSignals.insert(it->second.get());
	return true;
}

void Simulation::dump(ObserverFunc fn) {
	for (auto& is : wrappers) {
		const auto& s = *is.second;
		fn(s.as, s.value);
	}
}

void Simulation::addEvent(
	SimulationTime T,
	const AssemblySignal* signal,
	const SimulationValue& value) {

	auto it = wrappers.find(signal);
	if (it == wrappers.end())
		return;

	eventQueue.addEvent(SimulationEvent(T, it->second.get(), value));
}

void Simulation::step(ObserverFunc fn) {
	if (eventQueue.isAtEnd())
		return;
	T = eventQueue.nextTime();
	std::set<const AssemblySignal*> changed;
	eventQueue.nextEvents([&](const SimulationEvent& ev){
		if (ev.signal->value == ev.value)
			return;
		ev.signal->value = ev.value;
		fn(ev.signal->as, ev.value);
		changed.insert(ev.signal->as);
	});
	eventQueue.pop();

	SimulationTime Tn = T;
	Tn.delta++;

	// Re-evaluate everything that depends on the changed signals.
	for (auto& is : wrappers) {
		auto s = is.second.get();
		if (s->as->assignment) {
			auto ab = s->as->assignment.get();
			if (auto a = dynamic_cast<const AssemblyExprIdentity*>(ab)) {
				if (changed.count(a->op)) {
					eventQueue.addEvent(SimulationEvent(
						Tn, s, wrappers[a->op]->value));
				}
			}
			if (auto a = dynamic_cast<const AssemblyExprDelayed*>(ab)) {
				if (changed.count(a->op)) {
					SimulationTime Td = Tn;
					if (a->d > 0) {
						Td.ps += a->d;
						Td.delta = 0;
					}
					eventQueue.addEvent(SimulationEvent(
						Td, s, wrappers[a->op]->value));
				}
			}
		}
	}
}
