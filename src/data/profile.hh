#pragma once
#include "resource.hh"

namespace mcvm {
	// Represents the settings for a profile
	struct ProfileSettings {
		
	};

	// Base for profile
	class ProfileBase {
		Profile* parent = nullptr;

		public:
		ProfileSettings settings;
		MCVersion version;

		// Make sure that the profile has a cached rendered config
		void ensure_cached() {}
	};

	// A profile that also holds client-specific resources
	class Profile : public ProfileBase {
		// Resources
		std::vector<ResourceRef<WorldResource>> worlds;
	};

	class ServerProfile : public ProfileBase {
		// Resources
		std::vector<ResourceRef<PluginResource>> plugins;
		// A server can only have one world but we store multiple as well for
		// easy switching and bungeecord/multiverse and stuff
		std::vector<ResourceRef<WorldResource>> worlds;
		ResourceRef<WorldResource> current_world;
	};
};
